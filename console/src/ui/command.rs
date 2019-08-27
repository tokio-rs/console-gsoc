use crate::filter::*;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Modifier(Modifier),
    GroupBy(GroupBy),
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Command::from_str(s).ok_or(())
    }
}

impl Command {
    fn from_str(string: &str) -> Option<Command> {
        let command_end = string.find(char::is_whitespace).unwrap_or(string.len());
        let (command_str, remaining) = string.split_at(command_end);
        match command_str {
            _ if command_str.starts_with("event.") => Command::parse_event(command_str, remaining),
            _ => None,
        }
    }

    fn parse_event(command: &str, remaining: &str) -> Option<Command> {
        let mut segments = command.split('.');
        if segments.next() != Some("event") {
            return None;
        }
        match segments.next()? {
            "field" => {
                let fieldname = segments.next()?;
                Some(Command::Modifier(Command::parse_operator(
                    fieldname, remaining,
                )?))
            }
            "group_by" => {
                let segment = segments.next()?;
                let fieldname = segments.next()?.to_string();
                let parent_by_name = "parent_by_name(";
                let group_by = match segment {
                    "field" => GroupBy::Field(fieldname),
                    "span" if fieldname.starts_with(parent_by_name) => {
                        let rest = &fieldname[parent_by_name.len()..];
                        // TODO: Not pretty, but it works for now
                        let name_end = rest.chars().position(|c| c == ')')?;
                        let name = Command::parse_string(&rest[..name_end])?;
                        let criterion = match segments.next()? {
                            "field" => {
                                let fieldname = segments.next()?.to_string();
                                SpanCriterion::Field(fieldname)
                            }
                            "id" => SpanCriterion::Id,
                            _ => None?,
                        };
                        GroupBy::Span(SpanSelector::ParentByName { name, criterion })
                    }
                    "span" => {
                        let criterion = match fieldname.as_ref() {
                            "field" => {
                                let fieldname = segments.next()?.to_string();
                                SpanCriterion::Field(fieldname)
                            }
                            "id" => SpanCriterion::Id,
                            _ => None?,
                        };
                        GroupBy::Span(SpanSelector::SpanCriterion(criterion))
                    }
                    _ => None?,
                };
                Some(Command::GroupBy(group_by))
            }
            _ => return None,
        }
    }

    fn parse_operator(fieldname: &str, mut remaining: &str) -> Option<Modifier> {
        let fieldname = fieldname.to_string();
        // remaining: ' == "example"'
        remaining = remaining.trim();
        // remaining: '== "example"'
        let operator_end = remaining.find(char::is_whitespace)?;
        let (operator, mut remaining) = remaining.split_at(operator_end);
        // remaining: ' "example"'
        remaining = remaining.trim();
        // remaining = '"example"'
        let value = Command::parse_string(remaining)?;
        let modifier_ty = match operator {
            "==" => Modifier::equals,
            "matches" => Modifier::matches,
            "contains" => Modifier::contains,
            "starts_with" => Modifier::starts_with,
            _ => None?,
        };
        Some(modifier_ty(fieldname, value))
    }

    fn parse_string(string: &str) -> Option<String> {
        let mut chars = string.chars();
        if string.len() < 2 || chars.next() != Some('"') || chars.last() != Some('"') {
            None?
        }
        let inner = &string[1..string.len() - 1];
        Some(inner.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_string() {
        assert_eq!(
            Command::parse_string(r#""example""#),
            Some("example".to_string())
        );
    }

    #[test]
    fn parse_multi_string() {
        assert_eq!(
            Command::parse_string(r#""foo bar baz""#),
            Some("foo bar baz".to_string())
        );
    }

    #[test]
    fn parse_command_field() {
        assert_eq!(
            r#"event.field.message == "example""#.parse(),
            Ok(Command::Modifier(Modifier::equals(
                "message".to_string(),
                "example".to_string()
            )))
        )
    }

    #[test]
    fn parse_command_group_by() {
        assert_eq!(
            r#"event.group_by.field.foo"#.parse(),
            Ok(Command::GroupBy(GroupBy::Field("foo".to_string())))
        );

        assert_eq!(
            r#"event.group_by.span.field.foo"#.parse(),
            Ok(Command::GroupBy(GroupBy::Span(
                SpanSelector::SpanCriterion(SpanCriterion::Field("foo".to_string()))
            )))
        );

        assert_eq!(
            r#"event.group_by.span.parent_by_name("bar").field.foo"#.parse(),
            Ok(Command::GroupBy(GroupBy::Span(
                SpanSelector::ParentByName {
                    name: "bar".to_string(),
                    criterion: SpanCriterion::Field("foo".to_string())
                }
            )))
        );
    }

    #[test]
    fn fail_command_group_by() {
        assert_ne!(
            r#"event.group_by.field.foos"#.parse(),
            Ok(Command::GroupBy(GroupBy::Field("foo".to_string())))
        );

        assert_ne!(
            r#"event.group_by.span.field.foos"#.parse(),
            Ok(Command::GroupBy(GroupBy::Span(
                SpanSelector::SpanCriterion(SpanCriterion::Field("foo".to_string()))
            )))
        );

        assert_ne!(
            r#"event.group_by.span.parent_by_name("bars").field.foos"#.parse(),
            Ok(Command::GroupBy(GroupBy::Span(
                SpanSelector::ParentByName {
                    name: "bar".to_string(),
                    criterion: SpanCriterion::Field("foo".to_string())
                }
            )))
        );
    }
}
