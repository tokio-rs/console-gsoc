use crate::filter::*;

#[derive(Debug, PartialEq)]
pub(crate) enum Command {
    Event(Modifier),
}

impl Command {
    pub(crate) fn parse(string: &str) -> Option<Command> {
        let command_end = string.find(char::is_whitespace)?;
        let (command_str, remaining) = string.split_at(command_end);
        match command_str {
            _ if command_str.starts_with("event.") => Command::parse_event(command_str, remaining),
            _ => None,
        }
    }

    fn parse_event<'s>(command: &str, remaining: &str) -> Option<Command> {
        let mut segments = command.split('.');
        if !(segments.next() == Some("event") && segments.next() == Some("field")) {
            return None;
        }
        let fieldname = segments.next()?;
        Some(Command::Event(Command::parse_operator(
            fieldname, remaining,
        )?))
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

    fn parse_string<'s>(string: &str) -> Option<String> {
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
    fn parse_command() {
        assert_eq!(
            Command::parse(r#"event.field.message == "example""#),
            Some(Command::Event(Modifier::equals(
                "message".to_string(),
                "example".to_string()
            )))
        )
    }
}
