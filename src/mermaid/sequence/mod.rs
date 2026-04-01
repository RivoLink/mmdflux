//! Sequence diagram parser.
//!
//! Hand-written line-oriented parser for Mermaid sequence diagram syntax.
//! Supports participant/actor declarations, messages with all standard arrow
//! types, notes over one participant, and autonumber.

pub mod ast;

use ast::{
    ActivationModifier, ArrowHead, BlockDividerKind, BlockKind, LineStyle, NotePlacement,
    ParticipantKind, SequenceStatement,
};

use crate::errors::ParseDiagnostic;

/// Result of parsing a sequence diagram.
///
/// Contains the parsed statements and any warnings collected during parsing.
pub struct SequenceParseResult {
    /// Parsed statements in source order.
    pub statements: Vec<SequenceStatement>,
    /// Warnings collected during parsing (e.g., skipped lines).
    pub warnings: Vec<ParseDiagnostic>,
}

/// Parse a sequence diagram from Mermaid input text.
///
/// Expects the input to start with `sequenceDiagram` (case-insensitive).
/// Returns parsed statements and any warnings (e.g., unrecognized lines).
pub fn parse_sequence(
    input: &str,
) -> Result<SequenceParseResult, Box<dyn std::error::Error + Send + Sync>> {
    let mut statements = Vec::new();
    let mut warnings = Vec::new();
    let mut lines = input.lines().enumerate().peekable();

    // Skip frontmatter
    if let Some((_, first)) = lines.peek()
        && first.trim() == "---"
    {
        lines.next();
        for (_, line) in lines.by_ref() {
            if line.trim() == "---" {
                break;
            }
        }
    }

    // Skip leading comments and whitespace, then consume header
    let mut found_header = false;
    while let Some((_, line)) = lines.peek() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            lines.next();
            continue;
        }
        if trimmed.to_lowercase() == "sequencediagram" {
            found_header = true;
            lines.next();
            break;
        }
        return Err(format!("Expected 'sequenceDiagram' header, got: {trimmed}").into());
    }

    if !found_header {
        return Err("Missing 'sequenceDiagram' header".into());
    }

    // Parse body lines
    for (line_num, line) in lines {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("%%") {
            continue;
        }

        // Try each construct in order
        if trimmed.to_lowercase() == "autonumber" {
            statements.push(SequenceStatement::Autonumber);
            continue;
        }

        if let Some(stmt) = try_parse_block_start(trimmed) {
            statements.push(stmt);
            continue;
        }

        if let Some(stmt) = try_parse_block_divider(trimmed) {
            statements.push(stmt);
            continue;
        }

        if let Some(stmt) = try_parse_block_end(trimmed) {
            statements.push(stmt);
            continue;
        }

        if let Some(stmt) = try_parse_participant(trimmed) {
            statements.push(stmt);
            continue;
        }

        if let Some(stmt) = try_parse_activate(trimmed) {
            statements.push(stmt);
            continue;
        }

        if let Some(stmt) = try_parse_note(trimmed) {
            statements.push(stmt);
            continue;
        }

        if let Some(stmt) = try_parse_message(trimmed) {
            statements.push(stmt);
            continue;
        }

        // Permissive: skip unrecognized lines but collect a warning
        warnings.push(ParseDiagnostic::warning(
            Some(line_num + 1), // 1-indexed
            None,
            format!("skipped unrecognized line: {trimmed}"),
        ));
    }

    Ok(SequenceParseResult {
        statements,
        warnings,
    })
}

fn try_parse_block_start(line: &str) -> Option<SequenceStatement> {
    parse_keyword_line(line, "loop")
        .map(|label| SequenceStatement::BlockStart {
            kind: BlockKind::Loop,
            label,
        })
        .or_else(|| {
            parse_keyword_line(line, "alt").map(|label| SequenceStatement::BlockStart {
                kind: BlockKind::Alt,
                label,
            })
        })
        .or_else(|| {
            parse_keyword_line(line, "opt").map(|label| SequenceStatement::BlockStart {
                kind: BlockKind::Opt,
                label,
            })
        })
        .or_else(|| {
            parse_keyword_line(line, "par").map(|label| SequenceStatement::BlockStart {
                kind: BlockKind::Par,
                label,
            })
        })
        .or_else(|| {
            parse_keyword_line(line, "critical").map(|label| SequenceStatement::BlockStart {
                kind: BlockKind::Critical,
                label,
            })
        })
        .or_else(|| {
            parse_keyword_line(line, "break").map(|label| SequenceStatement::BlockStart {
                kind: BlockKind::Break,
                label,
            })
        })
}

fn try_parse_block_divider(line: &str) -> Option<SequenceStatement> {
    parse_keyword_line(line, "else")
        .map(|label| SequenceStatement::BlockDivider {
            kind: BlockDividerKind::Else,
            label,
        })
        .or_else(|| {
            parse_keyword_line(line, "and").map(|label| SequenceStatement::BlockDivider {
                kind: BlockDividerKind::And,
                label,
            })
        })
        .or_else(|| {
            parse_keyword_line(line, "option").map(|label| SequenceStatement::BlockDivider {
                kind: BlockDividerKind::Option,
                label,
            })
        })
}

fn try_parse_block_end(line: &str) -> Option<SequenceStatement> {
    if line.eq_ignore_ascii_case("end") {
        Some(SequenceStatement::BlockEnd)
    } else {
        None
    }
}

fn parse_keyword_line(line: &str, keyword: &str) -> Option<String> {
    let lower = line.to_lowercase();
    if lower == keyword {
        return Some(String::new());
    }

    if lower.starts_with(keyword) {
        let rest = &line[keyword.len()..];
        if rest.starts_with(char::is_whitespace) {
            return Some(rest.trim().to_string());
        }
    }

    None
}

/// Try to parse a `participant` or `actor` declaration.
fn try_parse_participant(line: &str) -> Option<SequenceStatement> {
    let lower = line.to_lowercase();
    let (kind, rest) = if lower.starts_with("participant ") || lower.starts_with("participant\t") {
        (ParticipantKind::Participant, &line["participant".len()..])
    } else if lower.starts_with("actor ") || lower.starts_with("actor\t") {
        (ParticipantKind::Actor, &line["actor".len()..])
    } else {
        return None;
    };

    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }

    // Check for alias: `participant A as Alice`
    let lower_rest = rest.to_lowercase();
    if let Some(as_pos) = lower_rest.find(" as ") {
        let id = rest[..as_pos].trim().to_string();
        let alias = rest[as_pos + 4..].trim().to_string();
        if !id.is_empty() && !alias.is_empty() {
            return Some(SequenceStatement::Participant {
                kind,
                id,
                alias: Some(alias),
            });
        }
    }

    Some(SequenceStatement::Participant {
        kind,
        id: rest.to_string(),
        alias: None,
    })
}

/// Try to parse an `activate <participant>` or `deactivate <participant>` statement.
fn try_parse_activate(line: &str) -> Option<SequenceStatement> {
    let lower = line.to_lowercase();
    if lower.starts_with("activate ") || lower.starts_with("activate\t") {
        let rest = line["activate".len()..].trim();
        if !rest.is_empty() {
            return Some(SequenceStatement::Activate {
                participant: rest.to_string(),
            });
        }
    } else if lower.starts_with("deactivate ") || lower.starts_with("deactivate\t") {
        let rest = line["deactivate".len()..].trim();
        if !rest.is_empty() {
            return Some(SequenceStatement::Deactivate {
                participant: rest.to_string(),
            });
        }
    }
    None
}

/// Try to parse a note statement.
///
/// Supports:
/// - `Note over A: text` (single participant)
/// - `Note over A,B: text` (spanning two participants)
/// - `Note left of A: text`
/// - `Note right of A: text`
fn try_parse_note(line: &str) -> Option<SequenceStatement> {
    let lower = line.to_lowercase();

    let (placement, rest) = if lower.starts_with("note left of ") {
        (NotePlacement::LeftOf, &line["note left of ".len()..])
    } else if lower.starts_with("note right of ") {
        (NotePlacement::RightOf, &line["note right of ".len()..])
    } else if lower.starts_with("note over ") {
        (NotePlacement::Over, &line["note over ".len()..])
    } else {
        return None;
    };

    let colon_pos = rest.find(':')?;
    let participant_str = rest[..colon_pos].trim();
    let text = rest[colon_pos + 1..].trim().to_string();

    if participant_str.is_empty() || text.is_empty() {
        return None;
    }

    let participants: Vec<String> = participant_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if participants.is_empty() {
        return None;
    }

    Some(SequenceStatement::Note {
        placement,
        participants,
        text,
    })
}

/// Arrow pattern entry: (syntax, line style, arrowhead).
struct ArrowPattern {
    syntax: &'static str,
    line_style: LineStyle,
    arrow_head: ArrowHead,
}

/// All supported arrow patterns, ordered by length (longest first) to prevent
/// prefix conflicts.
static ARROWS: &[ArrowPattern] = &[
    ArrowPattern {
        syntax: "-->>",
        line_style: LineStyle::Dashed,
        arrow_head: ArrowHead::Filled,
    },
    ArrowPattern {
        syntax: "->>",
        line_style: LineStyle::Solid,
        arrow_head: ArrowHead::Filled,
    },
    ArrowPattern {
        syntax: "-->",
        line_style: LineStyle::Dashed,
        arrow_head: ArrowHead::Open,
    },
    ArrowPattern {
        syntax: "->",
        line_style: LineStyle::Solid,
        arrow_head: ArrowHead::Open,
    },
    ArrowPattern {
        syntax: "--x",
        line_style: LineStyle::Dashed,
        arrow_head: ArrowHead::Cross,
    },
    ArrowPattern {
        syntax: "-x",
        line_style: LineStyle::Solid,
        arrow_head: ArrowHead::Cross,
    },
    ArrowPattern {
        syntax: "--)",
        line_style: LineStyle::Dashed,
        arrow_head: ArrowHead::Async,
    },
    ArrowPattern {
        syntax: "-)",
        line_style: LineStyle::Solid,
        arrow_head: ArrowHead::Async,
    },
];

/// Try to parse a message: `A->>B: text`, `A-->B: text`, `A-xB: text`, etc.
///
/// Also handles `+`/`-` activation shorthand: `A->>+B: text` activates B,
/// `B-->>-A: text` deactivates A.
fn try_parse_message(line: &str) -> Option<SequenceStatement> {
    for arrow in ARROWS {
        if let Some(arrow_pos) = line.find(arrow.syntax) {
            let from = line[..arrow_pos].trim().to_string();
            let rest = line[arrow_pos + arrow.syntax.len()..].trim();

            if from.is_empty() {
                continue;
            }

            // Check for +/- activation shorthand at the start of the target
            let (activate, rest) = if let Some(stripped) = rest.strip_prefix('+') {
                (Some(ActivationModifier::Activate), stripped)
            } else if let Some(stripped) = rest.strip_prefix('-') {
                (Some(ActivationModifier::Deactivate), stripped)
            } else {
                (None, rest)
            };

            // Split on first colon for "to: text"
            let (to, text) = if let Some(colon_pos) = rest.find(':') {
                let to = rest[..colon_pos].trim().to_string();
                let text = rest[colon_pos + 1..].trim().to_string();
                (to, text)
            } else {
                (rest.to_string(), String::new())
            };

            if to.is_empty() {
                continue;
            }

            return Some(SequenceStatement::Message {
                from,
                to,
                line_style: arrow.line_style,
                arrow_head: arrow.arrow_head,
                text,
                activate,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use ast::*;

    use super::*;

    /// Helper to parse and unwrap statements (ignoring warnings).
    fn parse_stmts(input: &str) -> Vec<SequenceStatement> {
        parse_sequence(input).unwrap().statements
    }

    #[test]
    fn parse_empty_diagram() {
        let stmts = parse_stmts("sequenceDiagram\n");
        assert!(stmts.is_empty());
    }

    #[test]
    fn parse_participant() {
        let stmts = parse_stmts("sequenceDiagram\nparticipant A");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Participant {
                kind: ParticipantKind::Participant,
                id: "A".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn parse_actor() {
        let stmts = parse_stmts("sequenceDiagram\nactor Bob");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Participant {
                kind: ParticipantKind::Actor,
                id: "Bob".to_string(),
                alias: None,
            }
        );
    }

    #[test]
    fn parse_participant_with_alias() {
        let stmts = parse_stmts("sequenceDiagram\nparticipant A as Alice");
        assert_eq!(
            stmts[0],
            SequenceStatement::Participant {
                kind: ParticipantKind::Participant,
                id: "A".to_string(),
                alias: Some("Alice".to_string()),
            }
        );
    }

    #[test]
    fn parse_actor_with_alias() {
        let stmts = parse_stmts("sequenceDiagram\nactor B as Bob");
        assert_eq!(
            stmts[0],
            SequenceStatement::Participant {
                kind: ParticipantKind::Actor,
                id: "B".to_string(),
                alias: Some("Bob".to_string()),
            }
        );
    }

    #[test]
    fn parse_additional_block_starts() {
        let stmts = parse_stmts(
            "sequenceDiagram\npar notifications\ncritical establish connection\nbreak success",
        );
        assert_eq!(
            stmts,
            vec![
                SequenceStatement::BlockStart {
                    kind: BlockKind::Par,
                    label: "notifications".to_string(),
                },
                SequenceStatement::BlockStart {
                    kind: BlockKind::Critical,
                    label: "establish connection".to_string(),
                },
                SequenceStatement::BlockStart {
                    kind: BlockKind::Break,
                    label: "success".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parse_additional_block_dividers() {
        let stmts = parse_stmts("sequenceDiagram\nand\noption Timeout");
        assert_eq!(
            stmts,
            vec![
                SequenceStatement::BlockDivider {
                    kind: BlockDividerKind::And,
                    label: String::new(),
                },
                SequenceStatement::BlockDivider {
                    kind: BlockDividerKind::Option,
                    label: "Timeout".to_string(),
                },
            ]
        );
    }

    #[test]
    fn parse_solid_filled_message() {
        let stmts = parse_stmts("sequenceDiagram\nA->>B: hello");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Filled,
                text: "hello".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_dashed_filled_message() {
        let stmts = parse_stmts("sequenceDiagram\nA-->>B: response");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Dashed,
                arrow_head: ArrowHead::Filled,
                text: "response".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_solid_open_message() {
        let stmts = parse_stmts("sequenceDiagram\nA->B: open");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Open,
                text: "open".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_dashed_open_message() {
        let stmts = parse_stmts("sequenceDiagram\nA-->B: dashed open");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Dashed,
                arrow_head: ArrowHead::Open,
                text: "dashed open".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_solid_cross_message() {
        let stmts = parse_stmts("sequenceDiagram\nA-xB: lost");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Cross,
                text: "lost".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_dashed_cross_message() {
        let stmts = parse_stmts("sequenceDiagram\nA--xB: dashed lost");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Dashed,
                arrow_head: ArrowHead::Cross,
                text: "dashed lost".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_solid_async_message() {
        let stmts = parse_stmts("sequenceDiagram\nA-)B: async");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Async,
                text: "async".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_dashed_async_message() {
        let stmts = parse_stmts("sequenceDiagram\nA--)B: dashed async");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Dashed,
                arrow_head: ArrowHead::Async,
                text: "dashed async".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_self_message() {
        let stmts = parse_stmts("sequenceDiagram\nA->>A: think");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "A".to_string(),
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Filled,
                text: "think".to_string(),
                activate: None,
            }
        );
    }

    #[test]
    fn parse_note_over() {
        let stmts = parse_stmts("sequenceDiagram\nNote over A: done");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Note {
                placement: ast::NotePlacement::Over,
                participants: vec!["A".to_string()],
                text: "done".to_string(),
            }
        );
    }

    #[test]
    fn parse_autonumber() {
        let stmts = parse_stmts("sequenceDiagram\nautonumber");
        assert_eq!(stmts.len(), 1);
        assert_eq!(stmts[0], SequenceStatement::Autonumber);
    }

    #[test]
    fn parse_interaction_operators() {
        let input = "\
sequenceDiagram
    alt available
        A->>B: Request
    else busy
        B->>A: Later
    end
    loop Every 5s
        A->>B: Retry
    end
    opt extra
        A->>A: Cache
    end";
        let stmts = parse_stmts(input);
        assert!(matches!(
            &stmts[0],
            SequenceStatement::BlockStart {
                kind: BlockKind::Alt,
                label
            } if label == "available"
        ));
        assert!(matches!(
            &stmts[2],
            SequenceStatement::BlockDivider {
                kind: BlockDividerKind::Else,
                label
            } if label == "busy"
        ));
        assert_eq!(stmts[4], SequenceStatement::BlockEnd);
        assert!(matches!(
            &stmts[5],
            SequenceStatement::BlockStart {
                kind: BlockKind::Loop,
                label
            } if label == "Every 5s"
        ));
        assert!(matches!(
            &stmts[8],
            SequenceStatement::BlockStart {
                kind: BlockKind::Opt,
                label
            } if label == "extra"
        ));
        assert_eq!(stmts.last(), Some(&SequenceStatement::BlockEnd));
    }

    #[test]
    fn parse_full_mvp_example() {
        let input = "\
sequenceDiagram
    autonumber
    participant A
    participant B
    A->>B: hello
    B-->>A: hi back
    A->>A: think
    Note over A: done";
        let stmts = parse_stmts(input);
        assert_eq!(stmts.len(), 7);
        assert_eq!(stmts[0], SequenceStatement::Autonumber);
        assert!(matches!(&stmts[1], SequenceStatement::Participant { id, .. } if id == "A"));
        assert!(matches!(&stmts[2], SequenceStatement::Participant { id, .. } if id == "B"));
        assert!(
            matches!(&stmts[3], SequenceStatement::Message { from, to, line_style: LineStyle::Solid, arrow_head: ArrowHead::Filled, .. } if from == "A" && to == "B")
        );
        assert!(
            matches!(&stmts[4], SequenceStatement::Message { from, to, line_style: LineStyle::Dashed, arrow_head: ArrowHead::Filled, .. } if from == "B" && to == "A")
        );
        assert!(
            matches!(&stmts[5], SequenceStatement::Message { from, to, .. } if from == "A" && to == "A")
        );
        assert!(
            matches!(&stmts[6], SequenceStatement::Note { participants, .. } if participants == &["A".to_string()])
        );
    }

    #[test]
    fn parse_skips_comments() {
        let input = "sequenceDiagram\n%% comment\nparticipant A";
        let stmts = parse_stmts(input);
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parse_skips_empty_lines() {
        let input = "sequenceDiagram\n\nparticipant A\n\nA->>B: hi";
        let stmts = parse_stmts(input);
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn parse_case_insensitive_header() {
        let stmts = parse_stmts("SEQUENCEDIAGRAM\nparticipant A");
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parse_missing_header_errors() {
        let result = parse_sequence("participant A\nA->>B: hi");
        assert!(result.is_err());
    }

    #[test]
    fn parse_skips_frontmatter() {
        let input = "---\ntitle: Test\n---\nsequenceDiagram\nparticipant A";
        let stmts = parse_stmts(input);
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn parse_note_case_insensitive() {
        let stmts = parse_stmts("sequenceDiagram\nnote over A: done");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], SequenceStatement::Note { .. }));
    }

    #[test]
    fn parse_message_without_text() {
        let stmts = parse_stmts("sequenceDiagram\nA->>B:");
        // Message with empty text after colon
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], SequenceStatement::Message { text, .. } if text.is_empty()));
    }

    #[test]
    fn parse_message_no_colon() {
        let stmts = parse_stmts("sequenceDiagram\nA->>B");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], SequenceStatement::Message { text, .. } if text.is_empty()));
    }

    #[test]
    fn parse_activate_deactivate_keywords() {
        let input = "sequenceDiagram\nactivate A\nparticipant B\ndeactivate A";
        let result = parse_sequence(input).unwrap();
        assert_eq!(result.statements.len(), 3);
        assert_eq!(result.warnings.len(), 0);
        assert_eq!(
            result.statements[0],
            SequenceStatement::Activate {
                participant: "A".to_string(),
            }
        );
        assert!(matches!(
            &result.statements[1],
            SequenceStatement::Participant { .. }
        ));
        assert_eq!(
            result.statements[2],
            SequenceStatement::Deactivate {
                participant: "A".to_string(),
            }
        );
    }

    #[test]
    fn parse_activation_shorthand_plus() {
        let stmts = parse_stmts("sequenceDiagram\nA->>+B: Request");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "A".to_string(),
                to: "B".to_string(),
                line_style: LineStyle::Solid,
                arrow_head: ArrowHead::Filled,
                text: "Request".to_string(),
                activate: Some(ActivationModifier::Activate),
            }
        );
    }

    #[test]
    fn parse_activation_shorthand_minus() {
        let stmts = parse_stmts("sequenceDiagram\nB-->>-A: Response");
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Message {
                from: "B".to_string(),
                to: "A".to_string(),
                line_style: LineStyle::Dashed,
                arrow_head: ArrowHead::Filled,
                text: "Response".to_string(),
                activate: Some(ActivationModifier::Deactivate),
            }
        );
    }

    #[test]
    fn parse_interaction_operators_do_not_warn() {
        let input = "sequenceDiagram\nloop Start\nparticipant B\nelse maybe\nend";
        let result = parse_sequence(input).unwrap();
        assert_eq!(result.warnings.len(), 0);
        assert_eq!(result.statements.len(), 4);
    }

    #[test]
    fn parse_permissive_skips_unknown_with_warnings() {
        let input = "sequenceDiagram\nunsupported start\nparticipant B\nunsupported branch\nend";
        let result = parse_sequence(input).unwrap();
        assert_eq!(result.statements.len(), 2);
        assert_eq!(result.warnings.len(), 2);
    }

    #[test]
    fn parse_arrow_priority_prevents_prefix_match() {
        // `-->>` should be matched as dashed-filled, not `-->` + `>`
        let stmts = parse_stmts("sequenceDiagram\nA-->>B: hi");
        assert!(matches!(
            &stmts[0],
            SequenceStatement::Message {
                line_style: LineStyle::Dashed,
                arrow_head: ArrowHead::Filled,
                ..
            }
        ));
    }

    #[test]
    fn parse_all_eight_arrow_types() {
        let input = "\
sequenceDiagram
    A->>B: filled solid
    A-->>B: filled dashed
    A->B: open solid
    A-->B: open dashed
    A-xB: cross solid
    A--xB: cross dashed
    A-)B: async solid
    A--)B: async dashed";
        let stmts = parse_stmts(input);
        assert_eq!(stmts.len(), 8);

        let expected = [
            (LineStyle::Solid, ArrowHead::Filled),
            (LineStyle::Dashed, ArrowHead::Filled),
            (LineStyle::Solid, ArrowHead::Open),
            (LineStyle::Dashed, ArrowHead::Open),
            (LineStyle::Solid, ArrowHead::Cross),
            (LineStyle::Dashed, ArrowHead::Cross),
            (LineStyle::Solid, ArrowHead::Async),
            (LineStyle::Dashed, ArrowHead::Async),
        ];

        for (i, (ls, ah)) in expected.iter().enumerate() {
            match &stmts[i] {
                SequenceStatement::Message {
                    line_style,
                    arrow_head,
                    ..
                } => {
                    assert_eq!(line_style, ls, "line_style mismatch at index {i}");
                    assert_eq!(arrow_head, ah, "arrow_head mismatch at index {i}");
                }
                other => panic!("expected Message at index {i}, got {other:?}"),
            }
        }
    }

    #[test]
    fn parse_note_left_of() {
        let stmts = parse_sequence("sequenceDiagram\nNote left of A: reminder")
            .unwrap()
            .statements;
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Note {
                placement: ast::NotePlacement::LeftOf,
                participants: vec!["A".to_string()],
                text: "reminder".to_string(),
            }
        );
    }

    #[test]
    fn parse_note_right_of() {
        let stmts = parse_sequence("sequenceDiagram\nNote right of B: status")
            .unwrap()
            .statements;
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Note {
                placement: ast::NotePlacement::RightOf,
                participants: vec!["B".to_string()],
                text: "status".to_string(),
            }
        );
    }

    #[test]
    fn parse_note_spanning() {
        let stmts = parse_sequence("sequenceDiagram\nNote over A,B: shared")
            .unwrap()
            .statements;
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Note {
                placement: ast::NotePlacement::Over,
                participants: vec!["A".to_string(), "B".to_string()],
                text: "shared".to_string(),
            }
        );
    }

    #[test]
    fn parse_note_spanning_with_spaces() {
        let stmts = parse_sequence("sequenceDiagram\nNote over A , B : spaced")
            .unwrap()
            .statements;
        assert_eq!(stmts.len(), 1);
        assert_eq!(
            stmts[0],
            SequenceStatement::Note {
                placement: ast::NotePlacement::Over,
                participants: vec!["A".to_string(), "B".to_string()],
                text: "spaced".to_string(),
            }
        );
    }

    #[test]
    fn parse_note_left_of_case_insensitive() {
        let stmts = parse_sequence("sequenceDiagram\nnote LEFT of A: test")
            .unwrap()
            .statements;
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            &stmts[0],
            SequenceStatement::Note {
                placement: ast::NotePlacement::LeftOf,
                ..
            }
        ));
    }

    #[test]
    fn parse_note_right_of_case_insensitive() {
        let stmts = parse_sequence("sequenceDiagram\nNOTE RIGHT OF A: test")
            .unwrap()
            .statements;
        assert_eq!(stmts.len(), 1);
        assert!(matches!(
            &stmts[0],
            SequenceStatement::Note {
                placement: ast::NotePlacement::RightOf,
                ..
            }
        ));
    }

    #[test]
    fn parse_block_keywords_case_insensitive() {
        let stmts = parse_sequence("sequenceDiagram\nALT First\nELSE Second\nEND")
            .unwrap()
            .statements;
        assert!(matches!(
            &stmts[0],
            SequenceStatement::BlockStart {
                kind: BlockKind::Alt,
                label
            } if label == "First"
        ));
        assert!(matches!(
            &stmts[1],
            SequenceStatement::BlockDivider {
                kind: BlockDividerKind::Else,
                label
            } if label == "Second"
        ));
        assert_eq!(stmts[2], SequenceStatement::BlockEnd);
    }
}
