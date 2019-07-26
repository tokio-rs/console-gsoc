use crate::storage::EventEntry;
use std::fmt::{Display, Formatter, Result};

use regex::Regex;

#[derive(Clone, Debug, Default)]
pub(crate) struct Filter {
    pub(crate) name: String,
    pub(crate) modifier: Vec<Modifier>,
}

impl Filter {
    pub(crate) fn insert_modifier(&mut self, modifier: Modifier) {
        for (i, m) in self.modifier.iter().enumerate() {
            if m.field_name() == modifier.field_name() {
                self.modifier[i] = modifier;
                return;
            }
        }
        self.modifier.push(modifier);
    }

    pub(crate) fn filter(&self, entry: &EventEntry) -> bool {
        self.modifier
            .iter()
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
                .greedy_by_name(name)
                .map(|string| string.starts_with(value)),
            Modifier::FieldEquals { name, value } => entry
                .event
                .greedy_by_name(name)
                .map(|string| &string == value),
            Modifier::FieldContains { name, value } => entry
                .event
                .greedy_by_name(name)
                .map(|string| string.contains(value)),
            Modifier::FieldMatches { name, regex } => entry
                .event
                .greedy_by_name(name)
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
