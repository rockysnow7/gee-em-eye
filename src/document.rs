use nom::{
    bytes::complete::{tag, take_till, take_until},
    character::complete::{none_of, space0, space1},
    combinator::{eof, opt},
    multi::{many0, many1},
    sequence::{preceded, terminated},
    IResult, Parser,
};

/// An element is a single unit of content in a document.
#[derive(Debug, PartialEq, Clone)]
pub enum Element {
    /// A line of text.
    Text(String),
    /// A link.
    Link {
        url: String,
        text: Option<String>,
    },
    /// A preformatted block of text.
    Preformatted {
        caption: Option<String>,
        lines: Vec<String>,
    },
    /// A heading.
    Heading {
        level: u8,
        text: String,
    },
    /// An unordered list item.
    UnorderedListItem(String),
    /// A quote.
    Quote(String),
}

// these are all parsers for enum variants
impl Element {
    fn text(input: &str) -> IResult<&str, Self> {
        many1(none_of("\n"))
        .parse(input)
        .map(|(input, chars)| {
            let line = chars.iter().collect::<String>();
            let line = line.trim().to_string();

            (input, Self::Text(line))
        })
    }

    fn link_without_text(input: &str) -> IResult<&str, Self> {
        preceded(
            (
                tag("=>"),
                space0,
            ),
            take_until("\n"),
        )
        .parse(input)
        .map(|(input, line)| (input, Self::Link {
            url: line.trim().to_string(),
            text: None,
        }))
    }

    fn link_with_text(input: &str) -> IResult<&str, Self> {
        preceded(
            (
                tag("=>"),
                space0,
            ),
            (
                take_till(|c: char| c.is_ascii_whitespace()),
                preceded(
                    space1,
                    take_until("\n"),
                ),
            ),
        )
        .parse(input)
        .map(|(input, (url, text))| (input, Self::Link {
            url: url.trim().to_string(),
            text: Some(text.trim().to_string()),
        }))
    }

    fn link(input: &str) -> IResult<&str, Self> {
        Self::link_with_text(input)
            .or(Self::link_without_text(input))
    }

    fn preformatted_start(input: &str) -> IResult<&str, Option<String>> {
        preceded(
            tag("```"),
            opt(preceded(
                space0,
                take_until("\n"),
            ))
        )
        .parse(input)
        .map(|(input, caption)| {
            let caption = caption.and_then(|c| {
                let trimmed = c.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            });
            (input, caption)
        })
    }

    fn preformatted(input: &str) -> IResult<&str, Self> {
        (
            terminated(
                Self::preformatted_start,
                tag("\n"),
            ),
            terminated(
                take_until("```"),
                tag("```"),
            ),
        )
        .parse(input)
        .map(|(input, (caption, content))| {
            let lines: Vec<String> = content.lines().map(String::from).collect();

            (input, Self::Preformatted {
                caption,
                lines,
            })
        })
    }

    fn heading(input: &str) -> IResult<&str, Self> {
        (
            many1(tag("#")),
            take_until("\n"),
        )
        .parse(input)
        .map(|(input, (level, text))| {
            (input, Self::Heading {
                level: level.len() as u8,
                text: text.trim().to_string(),
            })
        })
    }

    fn quote(input: &str) -> IResult<&str, Self> {
        preceded(
            tag(">"),
            take_until("\n"),
        )
        .parse(input)
        .map(|(input, text)| (input, Self::Quote(text.trim().to_string())))
    }

    fn unordered_list_item(input: &str) -> IResult<&str, Self> {
        preceded(
            tag("*"),
            take_until("\n"),
        )
        .parse(input)
        .map(|(input, text)| (input, Self::UnorderedListItem(text.trim().to_string())))
    }

    /// Parse an `Element` from a string.
    fn from_str(input: &str) -> IResult<&str, Self> {
        Self::preformatted(input)
            .or(Self::link(input))
            .or(Self::heading(input))
            .or(Self::unordered_list_item(input))
            .or(Self::quote(input))
            .or(Self::text(input))
    }
}

/// A `Document` is a collection of `Element`s.
#[derive(Debug, PartialEq)]
pub struct Document(pub Vec<Element>);

impl Document {
    /// Parse a `Document` from a string.
    fn from_str(input: &str) -> IResult<&str, Self> {
        let (input, elements) = terminated(many0(terminated(
            Element::from_str,
            many0(tag("\n")),
        )), eof).parse(input)?;

        Ok((input, Self(elements)))
    }
}

impl TryFrom<&str> for Document {
    type Error = String;

    fn try_from(input: &str) -> Result<Self, String> {
        let input = if !input.ends_with('\n') {
            format!("{}\n", input)
        } else {
            input.to_string()
        };

        let (_, document) = Document::from_str(&input).map_err(|e| e.to_string())?;

        Ok(document)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text() {
        let document = Document::try_from("Hello, world!");

        assert_eq!(document, Ok(Document(vec![
            Element::Text("Hello, world!".to_string()),
        ])));
    }

    #[test]
    fn link_without_text() {
        let document = Document::try_from("=> https://www.google.com");

        assert_eq!(document, Ok(Document(vec![
            Element::Link {
                url: "https://www.google.com".to_string(),
                text: None,
            },
        ])));
    }

    #[test]
    fn link_with_text() {
        let document = Document::try_from("=> https://www.google.com Hello, world!");

        assert_eq!(document, Ok(Document(vec![
            Element::Link {
                url: "https://www.google.com".to_string(),
                text: Some("Hello, world!".to_string()),
            },
        ])));
    }

    #[test]
    fn preformatted_without_caption() {
        let document = Document::try_from("```\nHello, world!\n\nYay.\n```");

        assert_eq!(document, Ok(Document(vec![
            Element::Preformatted {
                caption: None,
                lines: vec![
                    "Hello, world!".to_string(),
                    "".to_string(),
                    "Yay.".to_string(),
                ],
            },
        ])));
    }

    #[test]
    fn preformatted_with_caption() {
        let document = Document::try_from("``` Preformatted text\nHello, world!\n\nYay.\n```");

        assert_eq!(document, Ok(Document(vec![
            Element::Preformatted {
                caption: Some("Preformatted text".to_string()),
                lines: vec![
                    "Hello, world!".to_string(),
                    "".to_string(),
                    "Yay.".to_string(),
                ],
            },
        ])));
    }

    #[test]
    fn heading() {
        let document = Document::try_from("# Hello, world!\n## Yay.\n### Meow.");

        assert_eq!(document, Ok(Document(vec![
            Element::Heading { level: 1, text: "Hello, world!".to_string() },
            Element::Heading { level: 2, text: "Yay.".to_string() },
            Element::Heading { level: 3, text: "Meow.".to_string() },
        ])));
    }

    #[test]
    fn unordered_list_item() {
        let document = Document::try_from("* Hello, world!\n* Yay.\n* Meow.");

        assert_eq!(document, Ok(Document(vec![
            Element::UnorderedListItem("Hello, world!".to_string()),
            Element::UnorderedListItem("Yay.".to_string()),
            Element::UnorderedListItem("Meow.".to_string()),
        ])));
    }

    #[test]
    fn quote() {
        let document = Document::try_from("> Hello, world!");

        assert_eq!(document, Ok(Document(vec![
            Element::Quote("Hello, world!".to_string()),
        ])));
    }

    #[test]
    fn full_document() {
        let document = Document::try_from(r#"=> https://www.google.com Hello, world!

``` some rust code
fn main() {
    println!("Hello, world!");
}
```

## A list

* Hello, world!
* Yay."#);

        assert_eq!(document, Ok(Document(vec![
            Element::Link {
                url: "https://www.google.com".to_string(),
                text: Some("Hello, world!".to_string()),
            },
            Element::Preformatted {
                caption: Some("some rust code".to_string()),
                lines: vec![
                    "fn main() {".to_string(),
                    "    println!(\"Hello, world!\");".to_string(),
                    "}".to_string(),
                ] },
            Element::Heading { level: 2, text: "A list".to_string() },
            Element::UnorderedListItem("Hello, world!".to_string()),
            Element::UnorderedListItem("Yay.".to_string()),
        ])));
    }
}
