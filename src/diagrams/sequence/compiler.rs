//! Sequence diagram compiler.
//!
//! Compiles raw parsed AST statements into a validated `Sequence`.
//! Resolves participant references, assigns stable ordering, and
//! applies autonumbering.

use std::collections::HashMap;

use crate::mermaid::sequence::ast::{ActivationModifier, SequenceStatement};
use crate::timeline::sequence::model::{
    BlockDividerKind, BlockKind, NotePlacement, Participant, ParticipantKind, Sequence,
    SequenceEvent,
};

/// Compile parsed sequence statements into a validated model.
///
/// Participants are ordered by first appearance (explicit declarations first,
/// then implicit from message endpoints). Unknown participant references in
/// notes produce an error.
pub fn compile(
    statements: &[SequenceStatement],
) -> Result<Sequence, Box<dyn std::error::Error + Send + Sync>> {
    let mut participants: Vec<Participant> = Vec::new();
    let mut participant_index: HashMap<String, usize> = HashMap::new();
    let mut events: Vec<SequenceEvent> = Vec::new();
    let mut autonumber = false;
    let mut message_counter: usize = 0;
    let mut block_stack: Vec<BlockKind> = Vec::new();

    // First pass: collect explicit participant declarations (preserving order)
    for stmt in statements {
        if let SequenceStatement::Participant { kind, id, alias } = stmt
            && !participant_index.contains_key(id)
        {
            let idx = participants.len();
            participants.push(Participant {
                id: id.clone(),
                label: alias.as_deref().unwrap_or(id).to_string(),
                kind: kind.clone(),
            });
            participant_index.insert(id.clone(), idx);
        }
        if matches!(stmt, SequenceStatement::Autonumber) {
            autonumber = true;
        }
    }

    // Second pass: process messages and notes, creating implicit participants as needed
    for stmt in statements {
        match stmt {
            SequenceStatement::Message {
                from,
                to,
                line_style,
                arrow_head,
                text,
                activate,
            } => {
                let from_idx = ensure_participant(&mut participants, &mut participant_index, from);
                let to_idx = ensure_participant(&mut participants, &mut participant_index, to);

                message_counter += 1;
                events.push(SequenceEvent::Message {
                    from: from_idx,
                    to: to_idx,
                    line_style: *line_style,
                    arrow_head: *arrow_head,
                    text: text.clone(),
                    number: if autonumber {
                        Some(message_counter)
                    } else {
                        None
                    },
                });

                // Emit activation events from +/- shorthand.
                // + activates the target; - deactivates the source.
                match activate {
                    Some(ActivationModifier::Activate) => {
                        events.push(SequenceEvent::ActivateStart {
                            participant: to_idx,
                        });
                    }
                    Some(ActivationModifier::Deactivate) => {
                        events.push(SequenceEvent::ActivateEnd {
                            participant: from_idx,
                        });
                    }
                    None => {}
                }
            }
            SequenceStatement::Activate { participant: id } => {
                let idx = ensure_participant(&mut participants, &mut participant_index, id);
                events.push(SequenceEvent::ActivateStart { participant: idx });
            }
            SequenceStatement::Deactivate { participant: id } => {
                let idx = ensure_participant(&mut participants, &mut participant_index, id);
                events.push(SequenceEvent::ActivateEnd { participant: idx });
            }
            SequenceStatement::Note {
                placement,
                participants: names,
                text,
            } => {
                // Validate participant count for each placement
                match placement {
                    NotePlacement::LeftOf | NotePlacement::RightOf => {
                        if names.len() != 1 {
                            return Err(format!(
                                "Note left/right of requires exactly 1 participant, got {}",
                                names.len()
                            )
                            .into());
                        }
                    }
                    NotePlacement::Over => {
                        if names.len() > 2 {
                            return Err(format!(
                                "Note over supports at most 2 participants, got {}",
                                names.len()
                            )
                            .into());
                        }
                    }
                }

                let mut indices = Vec::with_capacity(names.len());
                for name in names {
                    let idx = participant_index.get(name.as_str()).copied().ok_or_else(
                        || -> Box<dyn std::error::Error + Send + Sync> {
                            format!("Note references unknown participant: {name}").into()
                        },
                    )?;
                    indices.push(idx);
                }

                events.push(SequenceEvent::Note {
                    placement: *placement,
                    participants: indices,
                    text: text.clone(),
                });
            }
            SequenceStatement::Participant { .. } | SequenceStatement::Autonumber => {
                // Already handled in first pass
            }
            SequenceStatement::BlockStart { kind, label } => {
                block_stack.push(*kind);
                events.push(SequenceEvent::BlockStart {
                    kind: *kind,
                    label: label.clone(),
                });
            }
            SequenceStatement::BlockDivider { kind, label } => {
                validate_block_divider(&block_stack, *kind)?;
                events.push(SequenceEvent::BlockDivider {
                    kind: *kind,
                    label: label.clone(),
                });
            }
            SequenceStatement::BlockEnd => {
                block_stack
                    .pop()
                    .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                        "encountered `end` without an open block".into()
                    })?;
                events.push(SequenceEvent::BlockEnd);
            }
        }
    }

    if !block_stack.is_empty() {
        let unclosed = block_stack
            .iter()
            .map(|kind| kind.keyword())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!("unclosed sequence block(s): {unclosed}").into());
    }

    Ok(Sequence {
        participants,
        events,
        autonumber,
    })
}

fn validate_block_divider(
    block_stack: &[BlockKind],
    divider: BlockDividerKind,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let Some(current) = block_stack.last() else {
        return Err(format!("encountered `{}` without an open block", divider.keyword()).into());
    };

    match (current, divider) {
        (BlockKind::Alt, BlockDividerKind::Else)
        | (BlockKind::Par, BlockDividerKind::And)
        | (BlockKind::Critical, BlockDividerKind::Option) => Ok(()),
        _ => Err(format!(
            "`{}` is not valid inside `{}` blocks",
            divider.keyword(),
            current.keyword()
        )
        .into()),
    }
}

/// Ensure a participant exists, creating an implicit one if needed.
/// Returns the participant index.
fn ensure_participant(
    participants: &mut Vec<Participant>,
    index: &mut HashMap<String, usize>,
    id: &str,
) -> usize {
    if let Some(&idx) = index.get(id) {
        return idx;
    }
    let idx = participants.len();
    participants.push(Participant {
        id: id.to_string(),
        label: id.to_string(),
        kind: ParticipantKind::Participant,
    });
    index.insert(id.to_string(), idx);
    idx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mermaid::sequence::parse_sequence;
    use crate::timeline::sequence::model::{ArrowHead, LineStyle};

    fn compile_input(input: &str) -> Sequence {
        let result = parse_sequence(input).unwrap();
        compile(&result.statements).unwrap()
    }

    #[test]
    fn compile_empty_diagram() {
        let model = compile_input("sequenceDiagram\n");
        assert!(model.participants.is_empty());
        assert!(model.events.is_empty());
        assert!(!model.autonumber);
    }

    #[test]
    fn compile_participants_in_order() {
        let model = compile_input("sequenceDiagram\nparticipant B\nparticipant A");
        assert_eq!(model.participants.len(), 2);
        assert_eq!(model.participants[0].id, "B");
        assert_eq!(model.participants[1].id, "A");
    }

    #[test]
    fn compile_participant_alias() {
        let model = compile_input("sequenceDiagram\nparticipant A as Alice");
        assert_eq!(model.participants[0].id, "A");
        assert_eq!(model.participants[0].label, "Alice");
    }

    #[test]
    fn compile_actor_kind() {
        let model = compile_input("sequenceDiagram\nactor B as Bob");
        assert_eq!(model.participants[0].kind, ParticipantKind::Actor);
    }

    #[test]
    fn compile_message_resolves_indices() {
        let model = compile_input("sequenceDiagram\nparticipant A\nparticipant B\nA->>B: hi");
        assert_eq!(model.events.len(), 1);
        match &model.events[0] {
            SequenceEvent::Message { from, to, .. } => {
                assert_eq!(*from, 0);
                assert_eq!(*to, 1);
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn compile_implicit_participants_from_messages() {
        let model = compile_input("sequenceDiagram\nA->>B: hi");
        assert_eq!(model.participants.len(), 2);
        assert_eq!(model.participants[0].id, "A");
        assert_eq!(model.participants[1].id, "B");
    }

    #[test]
    fn compile_explicit_before_implicit() {
        let model = compile_input("sequenceDiagram\nparticipant B\nA->>B: hi\nA->>C: hello");
        // B is explicit (index 0), A and C are implicit (1, 2)
        assert_eq!(model.participants[0].id, "B");
        assert_eq!(model.participants[1].id, "A");
        assert_eq!(model.participants[2].id, "C");
    }

    #[test]
    fn compile_self_message() {
        let model = compile_input("sequenceDiagram\nparticipant A\nA->>A: think");
        match &model.events[0] {
            SequenceEvent::Message { from, to, .. } => {
                assert_eq!(from, to);
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn compile_note_resolves_participant() {
        let model = compile_input("sequenceDiagram\nparticipant A\nNote over A: done");
        assert_eq!(model.events.len(), 1);
        match &model.events[0] {
            SequenceEvent::Note {
                placement,
                participants,
                text,
            } => {
                assert_eq!(*placement, NotePlacement::Over);
                assert_eq!(participants, &[0]);
                assert_eq!(text, "done");
            }
            _ => panic!("expected note"),
        }
    }

    #[test]
    fn compile_note_unknown_participant_errors() {
        let result = parse_sequence("sequenceDiagram\nNote over X: oops").unwrap();
        let compile_result = compile(&result.statements);
        assert!(compile_result.is_err());
        let err = compile_result.unwrap_err().to_string();
        assert!(err.contains("unknown participant"));
    }

    #[test]
    fn compile_autonumber() {
        let model = compile_input(
            "sequenceDiagram\nautonumber\nparticipant A\nparticipant B\nA->>B: first\nB->>A: second",
        );
        assert!(model.autonumber);
        match &model.events[0] {
            SequenceEvent::Message { number, .. } => assert_eq!(*number, Some(1)),
            _ => panic!("expected message"),
        }
        match &model.events[1] {
            SequenceEvent::Message { number, .. } => assert_eq!(*number, Some(2)),
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn compile_no_autonumber_no_numbers() {
        let model = compile_input("sequenceDiagram\nA->>B: hi");
        assert!(!model.autonumber);
        match &model.events[0] {
            SequenceEvent::Message { number, .. } => assert_eq!(*number, None),
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn compile_line_style_mapping() {
        let model = compile_input("sequenceDiagram\nA->>B: solid\nA-->>B: dashed");
        match &model.events[0] {
            SequenceEvent::Message {
                line_style,
                arrow_head,
                ..
            } => {
                assert_eq!(*line_style, LineStyle::Solid);
                assert_eq!(*arrow_head, ArrowHead::Filled);
            }
            _ => panic!("expected message"),
        }
        match &model.events[1] {
            SequenceEvent::Message {
                line_style,
                arrow_head,
                ..
            } => {
                assert_eq!(*line_style, LineStyle::Dashed);
                assert_eq!(*arrow_head, ArrowHead::Filled);
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn compile_all_arrow_heads() {
        let model =
            compile_input("sequenceDiagram\nA->>B: filled\nA->B: open\nA-xB: cross\nA-)B: async");
        let heads: Vec<_> = model
            .events
            .iter()
            .map(|e| match e {
                SequenceEvent::Message { arrow_head, .. } => *arrow_head,
                _ => panic!("expected message"),
            })
            .collect();
        assert_eq!(
            heads,
            vec![
                ArrowHead::Filled,
                ArrowHead::Open,
                ArrowHead::Cross,
                ArrowHead::Async
            ]
        );
    }

    #[test]
    fn compile_full_mvp() {
        let model = compile_input(
            "\
sequenceDiagram
    autonumber
    participant A as Alice
    participant B as Bob
    A->>B: hello
    B-->>A: hi back
    A->>A: think
    Note over A: done",
        );
        assert_eq!(model.participants.len(), 2);
        assert_eq!(model.participants[0].label, "Alice");
        assert_eq!(model.participants[1].label, "Bob");
        assert_eq!(model.events.len(), 4);
        assert!(model.autonumber);
    }

    #[test]
    fn compile_note_left_of() {
        let model =
            compile_input("sequenceDiagram\nparticipant A\nparticipant B\nNote left of A: left");
        match &model.events[0] {
            SequenceEvent::Note {
                placement,
                participants,
                text,
            } => {
                assert_eq!(*placement, NotePlacement::LeftOf);
                assert_eq!(participants, &[0]);
                assert_eq!(text, "left");
            }
            _ => panic!("expected note"),
        }
    }

    #[test]
    fn compile_note_right_of() {
        let model =
            compile_input("sequenceDiagram\nparticipant A\nparticipant B\nNote right of B: right");
        match &model.events[0] {
            SequenceEvent::Note {
                placement,
                participants,
                text,
            } => {
                assert_eq!(*placement, NotePlacement::RightOf);
                assert_eq!(participants, &[1]);
                assert_eq!(text, "right");
            }
            _ => panic!("expected note"),
        }
    }

    #[test]
    fn compile_note_spanning() {
        let model =
            compile_input("sequenceDiagram\nparticipant A\nparticipant B\nNote over A,B: spanning");
        match &model.events[0] {
            SequenceEvent::Note {
                placement,
                participants,
                text,
            } => {
                assert_eq!(*placement, NotePlacement::Over);
                assert_eq!(participants, &[0, 1]);
                assert_eq!(text, "spanning");
            }
            _ => panic!("expected note"),
        }
    }

    #[test]
    fn compile_note_spanning_unknown_participant_errors() {
        let stmts = parse_sequence("sequenceDiagram\nparticipant A\nNote over A,X: oops")
            .unwrap()
            .statements;
        let result = compile(&stmts);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("unknown participant"));
    }

    #[test]
    fn compile_interaction_operators_preserve_event_order() {
        let model = compile_input(
            "\
sequenceDiagram
    participant A
    participant B
    alt available
        A->>B: Request
    else busy
        B->>A: Retry later
    end",
        );
        assert!(matches!(
            &model.events[0],
            SequenceEvent::BlockStart {
                kind: BlockKind::Alt,
                label
            } if label == "available"
        ));
        assert!(matches!(&model.events[1], SequenceEvent::Message { .. }));
        assert!(matches!(
            &model.events[2],
            SequenceEvent::BlockDivider {
                kind: BlockDividerKind::Else,
                label
            } if label == "busy"
        ));
        assert!(matches!(&model.events[3], SequenceEvent::Message { .. }));
        assert_eq!(model.events[4], SequenceEvent::BlockEnd);
    }

    #[test]
    fn compile_else_outside_alt_errors() {
        let result = parse_sequence("sequenceDiagram\nloop retry\nelse nope\nend").unwrap();
        let err = compile(&result.statements).unwrap_err().to_string();
        assert!(err.contains("not valid inside `loop`"));
    }

    #[test]
    fn compile_unmatched_end_errors() {
        let result = parse_sequence("sequenceDiagram\nend").unwrap();
        let err = compile(&result.statements).unwrap_err().to_string();
        assert!(err.contains("without an open block"));
    }

    #[test]
    fn compile_unclosed_block_errors() {
        let result = parse_sequence("sequenceDiagram\nalt available").unwrap();
        let err = compile(&result.statements).unwrap_err().to_string();
        assert!(err.contains("unclosed sequence block"));
    }

    #[test]
    fn compile_par_and_preserve_event_order() {
        let model = compile_input(
            "\
sequenceDiagram
    participant Alice
    participant Bob
    participant Charlie
    par Notifications
        Alice->>Bob: Email
    and
        Alice->>Charlie: SMS
    end",
        );
        assert!(matches!(
            &model.events[0],
            SequenceEvent::BlockStart {
                kind: BlockKind::Par,
                label
            } if label == "Notifications"
        ));
        assert!(matches!(&model.events[1], SequenceEvent::Message { .. }));
        assert!(matches!(
            &model.events[2],
            SequenceEvent::BlockDivider {
                kind: BlockDividerKind::And,
                label
            } if label.is_empty()
        ));
        assert!(matches!(&model.events[3], SequenceEvent::Message { .. }));
        assert_eq!(model.events[4], SequenceEvent::BlockEnd);
    }

    #[test]
    fn compile_critical_option_preserve_event_order() {
        let model = compile_input(
            "\
sequenceDiagram
    participant Alice
    participant Bob
    critical Establish connection
        Alice->>Bob: Connect
    option Timeout
        Alice->>Alice: Retry
    end",
        );
        assert!(matches!(
            &model.events[0],
            SequenceEvent::BlockStart {
                kind: BlockKind::Critical,
                label
            } if label == "Establish connection"
        ));
        assert!(matches!(&model.events[1], SequenceEvent::Message { .. }));
        assert!(matches!(
            &model.events[2],
            SequenceEvent::BlockDivider {
                kind: BlockDividerKind::Option,
                label
            } if label == "Timeout"
        ));
        assert!(matches!(&model.events[3], SequenceEvent::Message { .. }));
        assert_eq!(model.events[4], SequenceEvent::BlockEnd);
    }

    #[test]
    fn compile_break_preserves_event_order() {
        let model = compile_input(
            "\
sequenceDiagram
    participant A
    participant B
    loop Retries
        A->>B: Try
        break Success
            B->>A: Done
        end
    end",
        );
        assert!(matches!(
            &model.events[2],
            SequenceEvent::BlockStart {
                kind: BlockKind::Break,
                label
            } if label == "Success"
        ));
        assert!(matches!(&model.events[3], SequenceEvent::Message { .. }));
        assert_eq!(model.events[4], SequenceEvent::BlockEnd);
        assert_eq!(model.events[5], SequenceEvent::BlockEnd);
    }

    #[test]
    fn compile_and_outside_par_errors() {
        let result = parse_sequence("sequenceDiagram\ncritical establish\nand\nend").unwrap();
        let err = compile(&result.statements).unwrap_err().to_string();
        assert!(err.contains("not valid inside `critical`"));
    }

    #[test]
    fn compile_option_outside_critical_errors() {
        let result = parse_sequence("sequenceDiagram\npar notify\noption fallback\nend").unwrap();
        let err = compile(&result.statements).unwrap_err().to_string();
        assert!(err.contains("not valid inside `par`"));
    }
}
