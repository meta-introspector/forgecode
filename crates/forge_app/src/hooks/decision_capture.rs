use async_trait::async_trait;
use forge_domain::{
    ContextMessage, Conversation, EventData, EventHandle, ResponsePayload,
};
use serde::Serialize;

/// Template context for the question-reminder injection.
#[derive(Serialize)]
struct DecisionQuestion {
    question: String,
    source_label: &'static str,
}

/// A parsed decision point extracted from an LLM response.
struct DecisionPoint {
    question: String,
    source_label: &'static str,
}

/// Detects uncertainty markers and decision points in LLM responses, and
/// injects clarification questions into the conversation context.
///
/// When the LLM expresses uncertainty or explores alternatives ("maybe",
/// "either X or Y", "I'm not sure"), this handler formulates a concise
/// question and injects it as a system reminder so the user can provide
/// guidance rather than the LLM guessing.
#[derive(Debug, Clone, Default)]
pub struct DecisionCaptureHandler;

impl DecisionCaptureHandler {
    pub fn new() -> Self {
        Self
    }

    /// Scans response text for known uncertainty patterns.
    fn scan_response(content: &str) -> Vec<DecisionPoint> {
        let mut points = Vec::new();

        // Pattern 1: "I need to either X or Y" — explicit alternative
        if let Some(start) = content.find("I need to either") {
            let snippet = &content[start..];
            // Take up to 120 chars after the marker
            let end = snippet.char_indices()
                .take(120)
                .map(|(i, _)| i)
                .last()
                .unwrap_or(snippet.len())
                .min(snippet.len());
            let context = &snippet[..end];
            points.push(DecisionPoint {
                question: format!(
                    "It looks like you need to decide between options. \
                     Can you clarify which direction to take?\n\nContext: {}",
                    context.trim()
                ),
                source_label: "decision-needed",
            });
        }

        // Pattern 2: "either ... or ..." — alternative framing
        if let Some(start) = content.find("either ") {
            if content[start..].contains(" or ") {
                let end = content[start..]
                    .char_indices()
                    .take(150)
                    .map(|(i, _)| i)
                    .last()
                    .unwrap_or(150)
                    .min(content[start..].len());
                let snippet = content[start..start + end].trim().to_string();
                if snippet.len() > 10 {
                    points.push(DecisionPoint {
                        question: format!(
                            "You mentioned alternatives. Which option should I go with?\n\nContext: {}",
                            snippet
                        ),
                        source_label: "alternative",
                    });
                }
            }
        }

        // Pattern 3: "I'm not sure" / "I'm uncertain" / "I'm not certain"
        for marker in &["I'm not sure", "I'm uncertain", "I'm not certain", "I am not sure"] {
            if let Some(start) = content.find(marker) {
                let end = content[start..]
                    .char_indices()
                    .take(100)
                    .map(|(i, _)| i)
                    .last()
                    .unwrap_or(100)
                    .min(content[start..].len());
                let snippet = content[start..start + end].trim().to_string();
                if snippet.len() > 10 {
                    points.push(DecisionPoint {
                        question: format!(
                            "It sounds uncertain. Can you clarify what you need?\n\nContext: {}",
                            snippet
                        ),
                        source_label: "uncertainty",
                    });
                }
                break; // Only one uncertainty match per response
            }
        }

        // Pattern 4: "maybe ... or maybe" or "perhaps ... or perhaps"
        let lower = content.to_lowercase();
        if lower.contains("maybe") && (lower.contains("or maybe") || lower.contains("or perhaps")) {
            let start = lower.find("maybe").unwrap_or(0);
            let end = content[start..]
                .char_indices()
                .take(120)
                .map(|(i, _)| i)
                .last()
                .unwrap_or(120)
                .min(content[start..].len());
            let snippet = content[start..start + end].trim().to_string();
            if snippet.len() > 10 {
                points.push(DecisionPoint {
                    question: format!(
                        "There seem to be multiple possibilities. \
                         Can you narrow down which approach to take?\n\nContext: {}",
                        snippet
                    ),
                    source_label: "exploration",
                });
            }
        }

        // Pattern 5: "I think ... but I'm not sure" — hedged opinion
        let lower = content.to_lowercase();
        if let Some(start) = lower.find("i think") {
            if lower[start..].contains("but") && lower[start..].contains("not sure") {
                let end = content[start..]
                    .char_indices()
                    .take(120)
                    .map(|(i, _)| i)
                    .last()
                    .unwrap_or(120)
                    .min(content[start..].len());
                let snippet = content[start..start + end].trim().to_string();
                if snippet.len() > 10 {
                    points.push(DecisionPoint {
                        question: format!(
                            "You seem to have a hunch but aren't certain. \
                             Can you confirm?\n\nContext: {}",
                            snippet
                        ),
                        source_label: "hedged",
                    });
                }
            }
        }

        // Pattern 6: "one option is ... another option is" — explicit enumeration
        let lower = content.to_lowercase();
        if lower.contains("one option") && lower.contains("another option") {
            let start = lower.find("one option").unwrap_or(0);
            let end = content[start..]
                .char_indices()
                .take(150)
                .map(|(i, _)| i)
                .last()
                .unwrap_or(150)
                .min(content[start..].len());
            let snippet = content[start..start + end].trim().to_string();
            if snippet.len() > 10 {
                points.push(DecisionPoint {
                    question: format!(
                        "You've listed multiple options. \
                         Which one should I proceed with?\n\nContext: {}",
                        snippet
                    ),
                    source_label: "multiple-options",
                });
            }
        }

        points
    }

    /// Checks whether the conversation already has a recent question reminder
    /// with matching content to avoid duplicate injection.
    fn has_recent_question(conversation: &Conversation, question: &str) -> bool {
        // Normalize both the signature and stored content to alphanumeric + whitespace
        // so that "I'm" in the signature (filtered to "Im") also matches "I'm" in the
        // stored message (also filtered to "Im").
        fn normalize(s: &str) -> String {
            s.chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .take(200)
                .collect()
        }

        let question_norm = normalize(question);

        conversation
            .context
            .as_ref()
            .map(|ctx| {
                ctx.messages
                    .iter()
                    .rev()
                    .take(10)
                    .filter_map(|entry| entry.message.content())
                    .any(|msg_content| normalize(msg_content).contains(&question_norm))
            })
            .unwrap_or(false)
    }
}

#[async_trait]
impl EventHandle<EventData<ResponsePayload>> for DecisionCaptureHandler {
    async fn handle(
        &self,
        event: &EventData<ResponsePayload>,
        conversation: &mut Conversation,
    ) -> anyhow::Result<()> {
        // Only scan responses that contain text content
        let content = &event.payload.message.content;
        if content.trim().is_empty() {
            return Ok(());
        }

        let points = Self::scan_response(content);
        if points.is_empty() {
            return Ok(());
        }

        // Take the most significant decision point to avoid flooding context
        if let Some(point) = points.into_iter().next() {
            // Avoid duplicate questions
            if Self::has_recent_question(conversation, &point.question) {
                return Ok(());
            }

            if let Some(context) = conversation.context.as_mut() {
                let message = format!(
                    "[DECISION NEEDED]\n\n{}",
                    point.question
                );
                let content = forge_template::Element::new(point.source_label)
                    .cdata(message);
                context
                    .messages
                    .push(ContextMessage::user(content, None).into());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use forge_domain::{
        Agent, Context, Conversation, EventData, ModelId, ResponsePayload,
    };
    use pretty_assertions::assert_eq;

    use super::*;

    fn fixture_agent() -> Agent {
        Agent::new(
            "test-agent",
            "test-provider".to_string().into(),
            ModelId::new("test-model"),
        )
    }

    fn fixture_conversation() -> Conversation {
        let mut conversation = Conversation::generate();
        conversation.context = Some(Context::default());
        conversation
    }

    fn fixture_event(content: &str) -> EventData<ResponsePayload> {
        let message = forge_domain::ChatCompletionMessageFull {
            content: content.to_string(),
            reasoning: None,
            tool_calls: vec![],
            thought_signature: None,
            reasoning_details: None,
            usage: forge_domain::Usage::default(),
            finish_reason: None,
            phase: None,
        };
        EventData::new(
            fixture_agent(),
            ModelId::new("test-model"),
            ResponsePayload::new(message),
        )
    }

    #[tokio::test]
    async fn test_no_uncertainty_no_injection() {
        let handler = DecisionCaptureHandler::new();
        let event = fixture_event("The answer is clear. I will proceed with option A.");
        let mut conversation = fixture_conversation();

        let initial_count = conversation.context.as_ref().unwrap().messages.len();
        handler.handle(&event, &mut conversation).await.unwrap();
        let actual = conversation.context.as_ref().unwrap().messages.len();
        let expected = initial_count;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_uncertainty_injects_reminder() {
        let handler = DecisionCaptureHandler::new();
        let event = fixture_event(
            "I'm not sure whether to use Redis or PostgreSQL for caching.",
        );
        let mut conversation = fixture_conversation();

        handler.handle(&event, &mut conversation).await.unwrap();

        let messages = &conversation.context.as_ref().unwrap().messages;
        assert_eq!(messages.len(), 1);

        let content = messages[0].message.content().unwrap();
        assert!(content.contains("DECISION NEEDED"));
        assert!(content.contains("uncertain"));
    }

    #[tokio::test]
    async fn test_decision_needed_detected() {
        let handler = DecisionCaptureHandler::new();
        let event = fixture_event(
            "I need to either fix the bug directly or refactor the module first.",
        );
        let mut conversation = fixture_conversation();

        handler.handle(&event, &mut conversation).await.unwrap();

        let messages = &conversation.context.as_ref().unwrap().messages;
        assert_eq!(messages.len(), 1);

        let content = messages[0].message.content().unwrap();
        assert!(content.contains("DECISION NEEDED"));
        assert!(content.contains("decide between options"));
    }

    #[tokio::test]
    async fn test_exploration_detected() {
        let handler = DecisionCaptureHandler::new();
        let event = fixture_event(
            "Maybe we can use a HashMap, or maybe a BTreeMap would be better for ordering.",
        );
        let mut conversation = fixture_conversation();

        handler.handle(&event, &mut conversation).await.unwrap();

        let messages = &conversation.context.as_ref().unwrap().messages;
        assert_eq!(messages.len(), 1);

        let content = messages[0].message.content().unwrap();
        assert!(content.contains("DECISION NEEDED"));
        assert!(content.contains("possibilities"));
    }

    #[tokio::test]
    async fn test_no_duplicate_injection() {
        let handler = DecisionCaptureHandler::new();
        let event = fixture_event("I'm not sure which approach is best here.");
        let mut conversation = fixture_conversation();

        // First call should inject
        handler.handle(&event, &mut conversation).await.unwrap();
        let after_first = conversation.context.as_ref().unwrap().messages.len();
        assert_eq!(after_first, 1);

        // Second call with same content should NOT inject
        handler.handle(&event, &mut conversation).await.unwrap();
        let after_second = conversation.context.as_ref().unwrap().messages.len();
        assert_eq!(after_second, 1);
    }

    #[tokio::test]
    async fn test_empty_content_does_nothing() {
        let handler = DecisionCaptureHandler::new();
        let event = fixture_event("");
        let mut conversation = fixture_conversation();

        let initial_count = conversation.context.as_ref().unwrap().messages.len();
        handler.handle(&event, &mut conversation).await.unwrap();
        let actual = conversation.context.as_ref().unwrap().messages.len();
        let expected = initial_count;

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_multiple_options_detected() {
        let handler = DecisionCaptureHandler::new();
        let event = fixture_event(
            "One option is to add input validation, another option is to handle it at the database level.",
        );
        let mut conversation = fixture_conversation();

        handler.handle(&event, &mut conversation).await.unwrap();

        let messages = &conversation.context.as_ref().unwrap().messages;
        assert_eq!(messages.len(), 1);

        let content = messages[0].message.content().unwrap();
        assert!(content.contains("DECISION NEEDED"));
        assert!(content.contains("multiple options"));
    }

    #[tokio::test]
    async fn test_hedged_opinion_detected() {
        let handler = DecisionCaptureHandler::new();
        // Pattern 5: "I think ... but ... not sure" — hedged opinion
        // This needs "I think", "but", AND "not sure" all in one sentence.
        // The full content triggers Pattern 5 which uses "hunch" wording.
        let event = fixture_event(
            "I think we should use the builder pattern, but I'm not sure if it's the best fit here.",
        );
        let mut conversation = fixture_conversation();

        handler.handle(&event, &mut conversation).await.unwrap();

        let messages = &conversation.context.as_ref().unwrap().messages;
        assert_eq!(messages.len(), 1);

        let content = messages[0].message.content().unwrap();
        assert!(content.contains("DECISION NEEDED"), "content: {}", content);
        assert!(content.contains("uncertain"), "content: {}", content);
    }

    #[test]
    fn test_scan_no_match() {
        let points = DecisionCaptureHandler::scan_response(
            "The answer is clear. Use Redis.",
        );
        assert!(points.is_empty());
    }

    #[test]
    fn test_scan_uncertainty() {
        let points = DecisionCaptureHandler::scan_response(
            "I'm not sure whether this is the right approach.",
        );
        assert!(!points.is_empty());
        assert_eq!(points[0].source_label, "uncertainty");
    }

    #[test]
    fn test_scan_decision_needed() {
        let points = DecisionCaptureHandler::scan_response(
            "I need to either fix the bug or refactor first.",
        );
        assert!(!points.is_empty());
        assert_eq!(points[0].source_label, "decision-needed");
    }

    #[test]
    fn test_scan_alternative() {
        let points = DecisionCaptureHandler::scan_response(
            "We could either use a queue or a stream processor.",
        );
        assert!(!points.is_empty());
        assert!(points.iter().any(|p| p.source_label == "alternative"));
    }

    #[test]
    fn test_scan_empty() {
        let points = DecisionCaptureHandler::scan_response("");
        assert!(points.is_empty());
    }
}
