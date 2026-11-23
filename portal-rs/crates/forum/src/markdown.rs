use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};
use url::Url;

/// Render only the supported Markdown events. Raw HTML is always rendered as text.
pub fn render(source: &str) -> String {
    let mut links = Vec::new();
    let events = Parser::new_ext(source, Options::empty()).filter_map(|event| match event {
        Event::Start(Tag::Link { dest_url, .. }) => {
            let url = Url::parse(&dest_url).ok().filter(|url| {
                matches!(url.scheme(), "http" | "https")
                    && url.host().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
            });
            links.push(url.is_some());
            url.map(|url| {
                let escaped = escape(url.as_str());
                Event::Html(format!(r#"<a href="{escaped}" rel="nofollow ugc">"#).into())
            })
        }
        Event::End(TagEnd::Link) => links
            .pop()
            .filter(|valid| *valid)
            .map(|_| Event::Html("</a>".into())),
        Event::Html(text) | Event::InlineHtml(text) => Some(Event::Text(text)),
        Event::Start(
            Tag::Paragraph
            | Tag::BlockQuote(_)
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::Emphasis
            | Tag::Strong,
        ) => {
            // Code fence info is display metadata supplied by the author.
            if matches!(event, Event::Start(Tag::CodeBlock(_))) {
                Some(Event::Start(Tag::CodeBlock(
                    pulldown_cmark::CodeBlockKind::Indented,
                )))
            } else {
                Some(event)
            }
        }
        Event::End(
            TagEnd::Paragraph
            | TagEnd::BlockQuote(_)
            | TagEnd::CodeBlock
            | TagEnd::List(_)
            | TagEnd::Item
            | TagEnd::Emphasis
            | TagEnd::Strong,
        )
        | Event::Text(_)
        | Event::Code(_)
        | Event::SoftBreak
        | Event::HardBreak
        | Event::Rule => Some(event),
        _ => None,
    });
    let mut output = String::new();
    html::push_html(&mut output, events);
    output
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_the_supported_markdown() {
        let rendered = render(
            "**bold** and *emphasis* with `code`\n\n> quote\n\n- one\n- two\n\n[link](https://example.com/?a=1&b=2)",
        );
        assert!(rendered.contains("<strong>bold</strong>"));
        assert!(rendered.contains("<em>emphasis</em>"));
        assert!(rendered.contains("<code>code</code>"));
        assert!(rendered.contains("<blockquote>"));
        assert!(rendered.contains("<li>two</li>"));
        assert!(
            rendered.contains(
                r#"<a href="https://example.com/?a=1&amp;b=2" rel="nofollow ugc">link</a>"#
            )
        );
    }

    #[test]
    fn preserves_text_for_unsupported_markup() {
        let rendered =
            render("# Heading\n\n<i>text</i> ![image](https://example.com/a) [relative](/path)");
        assert!(rendered.contains("Heading"));
        assert!(rendered.contains("&lt;i&gt;text&lt;/i&gt;"));
        assert!(rendered.contains("image"));
        assert!(rendered.contains("relative"));
        for tag in ["<h1", "<i>", "<img", "<a "] {
            assert!(!rendered.contains(tag));
        }
    }
}
