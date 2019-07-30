use crate::storage::EventEntry;

use std::fmt::{Display, Formatter, Result};

use indexmap::IndexMap;

use regex::Regex;

#[derive(Clone, Debug, Default)]
pub(crate) struct Filter {
    pub(crate) name: String,
    pub(crate) modifier: IndexMap<String, Modifier>,
}

impl Filter {
    pub(crate) fn insert_modifier(&mut self, modifier: Modifier) {
        self.modifier.insert(
            modifier
                .field_name()
                .expect("BUG: No field name found!")
                .to_string(),
            modifier,
        );
    }

    pub(crate) fn filter(&self, entry: &EventEntry) -> bool {
        self.modifier
            .values()
            .all(|m| m.filter(entry).unwrap_or(false))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Modifier {
    // Field methods
    FieldContains { name: String, value: String },
    FieldEquals { name: String, value: String },
    // TODO: Move regex instance into enum
    FieldMatches { name: String, regex: String },
    FieldStartsWith { name: String, value: String },
}

impl Display for Modifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Modifier::FieldContains { name, value } => {
                write!(f, "event.field.{} contains \"{}\"", name, value)
            }
            Modifier::FieldEquals { name, value } => {
                write!(f, "event.field.{} == \"{}\"", name, value)
            }
            Modifier::FieldMatches { name, regex } => {
                write!(f, "event.field.{} matches \"{}\"", name, regex)
            }
            Modifier::FieldStartsWith { name, value } => {
                write!(f, "event.field.{} starts_with \"{}\"", name, value)
            }
        }
    }
}

impl Modifier {
    fn field_name(&self) -> Option<&str> {
        match self {
            Modifier::FieldContains { name, .. } => Some(&name),
            Modifier::FieldEquals { name, .. } => Some(&name),
            Modifier::FieldMatches { name, .. } => Some(&name),
            Modifier::FieldStartsWith { name, .. } => Some(&name),
        }
    }

    fn filter(&self, entry: &EventEntry) -> Option<bool> {
        match self {
            Modifier::FieldStartsWith { name, value } => entry
                .event
                .any_by_name(name)
                .map(|string| string.starts_with(value)),
            Modifier::FieldEquals { name, value } => {
                entry.event.any_by_name(name).map(|string| &string == value)
            }
            Modifier::FieldContains { name, value } => entry
                .event
                .any_by_name(name)
                .map(|string| string.contains(value)),
            Modifier::FieldMatches { name, regex } => entry
                .event
                .any_by_name(name)
                .and_then(|string| Regex::new(regex).ok().map(|re| re.is_match(&string))),
        }
    }

    pub(crate) fn equals(name: String, value: String) -> Modifier {
        Modifier::FieldEquals { name, value }
    }

    pub(crate) fn contains(name: String, value: String) -> Modifier {
        Modifier::FieldContains { name, value }
    }

    pub(crate) fn starts_with(name: String, value: String) -> Modifier {
        Modifier::FieldStartsWith { name, value }
    }

    pub(crate) fn matches(name: String, regex: String) -> Modifier {
        Modifier::FieldMatches { name, regex }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::storage::*;

    fn event_entry() -> EventEntry {
        let mut event = Event::default();
        event.values.push(Value {
            field: Some(Field {
                name: "foo".to_string(),
            }),
            value: Some(value::Value::Str("barbazboz".to_string())),
        });
        EventEntry { span: None, event }
    }

    #[test]
    fn modifier_equals() {
        let entry = event_entry();

        let doesnt_exist = Modifier::equals("blah".to_string(), "example".to_string());
        assert_eq!(doesnt_exist.filter(&entry), None);

        let not_equal = Modifier::equals("foo".to_string(), "example".to_string());
        assert_eq!(not_equal.filter(&entry), Some(false));

        let equals = Modifier::equals("foo".to_string(), "barbazboz".to_string());
        assert_eq!(equals.filter(&entry), Some(true));
    }

    #[test]
    fn modifier_contains() {
        let entry = event_entry();

        let not_contained = Modifier::contains("foo".to_string(), "example".to_string());
        assert_eq!(not_contained.filter(&entry), Some(false));

        let contained = Modifier::contains("foo".to_string(), "baz".to_string());
        assert_eq!(contained.filter(&entry), Some(true));
    }

    #[test]
    fn modifier_regex() {
        let entry = event_entry();

        let no_match = Modifier::matches("foo".to_string(), "example".to_string());
        assert_eq!(no_match.filter(&entry), Some(false));

        let matches = Modifier::matches("foo".to_string(), "b[aeiou]z".to_string());
        assert_eq!(matches.filter(&entry), Some(true));
    }

    #[test]
    fn modifier_starts_with() {
        let entry = event_entry();

        let no_match = Modifier::starts_with("foo".to_string(), "example".to_string());
        assert_eq!(no_match.filter(&entry), Some(false));

        let matches = Modifier::starts_with("foo".to_string(), "bar".to_string());
        assert_eq!(matches.filter(&entry), Some(true));
    }
}
