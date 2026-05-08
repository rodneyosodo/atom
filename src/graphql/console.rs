use axum::response::Html;

use super::{console_css, console_html as html, console_js};

pub async fn graphql_console() -> Html<String> {
    Html(console_html())
}

pub(crate) fn console_html() -> String {
    let capacity = html::HTML_BEFORE_CSS.len()
        + console_css::CONSOLE_CSS.len()
        + html::HTML_AFTER_CSS_BEFORE_JS.len()
        + console_js::CONSOLE_JS.len()
        + html::HTML_AFTER_JS.len();
    let mut page = String::with_capacity(capacity);
    page.push_str(html::HTML_BEFORE_CSS);
    page.push_str(console_css::CONSOLE_CSS);
    page.push_str(html::HTML_AFTER_CSS_BEFORE_JS);
    page.push_str(console_js::CONSOLE_JS);
    page.push_str(html::HTML_AFTER_JS);
    page
}

#[cfg(test)]
mod tests {
    use super::console_html;

    #[test]
    fn console_html_contains_expected_sections() {
        let html = console_html();

        for text in [
            "Atom API Builder",
            "What do you want to do?",
            "Operation Builder",
            "Advanced GraphQL",
            "API Builder",
            "API Templates",
            "Queries",
            "Mutations",
            "Field selector",
            "Variables",
            "Save as template",
            "Copy curl",
            "Copy JavaScript",
            "Profile picker",
            "Schema form",
            "Attributes JSON",
            "Derived kind badge",
            "mutation CreateEntity",
            "WHO",
            "CAN DO",
            "ON",
            "WHEN",
            "EFFECT",
            "allow",
            "deny",
            "createPolicy",
            "Save as API template",
            "AI Assistant",
            "Generate prompt",
            "GraphQL operation",
            "variables JSON",
            "safety notes",
            "API Endpoint Builder",
            "Custom API Endpoint",
            "Runs saved GraphQL template",
        ] {
            assert!(html.contains(text), "missing {text}");
        }
    }

    #[test]
    fn console_html_uses_generic_atom_operations_only() {
        let html = console_html();

        for operation in [
            "createDomain",
            "createClient",
            "createChannel",
            "connectClientToChannel",
        ] {
            assert!(!html.contains(operation), "unexpected {operation}");
        }
    }

    #[test]
    fn console_entity_builder_is_profile_aware() {
        let html = console_html();

        for text in [
            "profiles(objectKind: \"entity\"",
            "profileVersions(profileId: $profileId)",
            "if (!input.profileId)",
            "input.kind = $(config.kind || \"entityKind\").value;",
            "Created entity <strong>",
            "Internal Atom kind is <strong>",
            "Profile is <strong>",
        ] {
            assert!(html.contains(text), "missing {text}");
        }
    }

    #[test]
    fn console_policy_builder_is_visual_and_generic() {
        let html = console_html();

        for text in [
            "Subject kind",
            "Grant kind",
            "Scope kind",
            "Conditions JSON",
            "Resource or entity",
            "mutation CreatePolicy",
            "copyPolicyCurl",
            "copyPolicyJs",
            "savePolicyTemplate",
        ] {
            assert!(html.contains(text), "missing {text}");
        }
    }

    #[test]
    fn console_assistant_generates_prompts_only() {
        let html = console_html();
        let start = html.find(r#"<section id="screen-assistant""#).unwrap();
        let end = start + html[start..].find("</section>").unwrap();
        let assistant = &html[start..end];

        for text in [
            "AI Assistant",
            "User request",
            "Selected tenant/profile/resource context",
            "Include schema summary",
            "Include saved templates",
            "Include current operation",
            "Create a tenant and a device entity",
            "Create a protected resource and allow an entity to access it",
            "Explain why an authorization check failed",
            "Generate a reusable API template",
            "Generate prompt",
            "GraphQL operation",
            "variables JSON",
            "safety notes",
        ] {
            assert!(assistant.contains(text), "missing {text}");
        }

        for forbidden in ["OpenAI", "Anthropic", "provider", "API key", "api key"] {
            assert!(
                !assistant.contains(forbidden),
                "assistant references {forbidden}"
            );
        }
    }

    #[test]
    fn console_endpoint_builder_is_generic() {
        let html = console_html();

        for text in [
            "API Endpoint Builder",
            "Custom API Endpoint",
            "Runs saved GraphQL template",
            "Choose template",
            "Configure route",
            "Map request to variables",
            "Test and publish",
            "Endpoint list",
            "View logs",
            "Copy curl",
            "Copy JavaScript fetch",
            "caller_context",
            "service_context",
            "/api/custom/",
            "endpointVariablesMapping",
            "endpointRequestSchema",
            "previewApiEndpoint",
            "enableApiEndpoint",
            "disableApiEndpoint",
        ] {
            assert!(html.contains(text), "missing {text}");
        }
    }
}
