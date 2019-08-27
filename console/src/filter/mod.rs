use crate::{
    storage::{messages, EventEntry, Span, Store},
    ui::command::Command,
};

use std::fmt::{self, Display, Formatter};

use indexmap::IndexMap;
use itertools::Itertools;

use messages::{value::Value, ValueContainer};
use regex::Regex;

#[derive(Clone, PartialEq)]
pub(crate) enum Entries {
    Grouped {
        group_by: GroupBy,
        groups: Vec<(Option<Value>, Entries)>,
    },
    Entries(Vec<usize>),
}

impl Entries {
    pub fn len(&self) -> usize {
        match self {
            Entries::Entries(entries) => entries.len(),
            Entries::Grouped { groups, .. } => {
                groups
                    .iter()
                    .map(|(_, entries)| entries.len())
                    .sum::<usize>()
                    + 1
            }
        }
    }

    fn retain(&mut self, f: impl Fn(&usize) -> bool + Copy) {
        match self {
            Entries::Entries(vec) => vec.retain(f),
            Entries::Grouped { groups, .. } => {
                groups.iter_mut().for_each(|(_, entries)| entries.retain(f))
            }
        }
    }

    fn group(self, key: impl Fn(&usize) -> Option<Value> + Copy, group_by: GroupBy) -> Entries {
        match self {
            Entries::Grouped { .. } => unimplemented!("Nested groups are not yet supported"),
            Entries::Entries(mut entries) => {
                entries.sort_by_key(key);
                let groups = entries
                    .into_iter()
                    .filter(|id| key(id).is_some())
                    .group_by(key)
                    .into_iter()
                    .map(|(value, indices)| -> (Option<Value>, Entries) {
                        (value, Entries::Entries(indices.collect()))
                    })
                    .collect();
                Entries::Grouped { group_by, groups }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct Filter {
    pub(crate) name: String,
    pub(crate) modifier: IndexMap<String, Modifier>,
    pub(crate) group_by: Option<GroupBy>,
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

    pub(crate) fn group(&mut self, group_by: GroupBy) {
        self.group_by = Some(group_by);
    }

    pub(crate) fn apply<'i>(&'i self, raw: &'i Store, mut entries: Entries) -> Entries {
        entries.retain(|e| {
            let event = &raw.events()[*e];
            self.modifier
                .values()
                .all(|m| m.filter(event).unwrap_or(false))
        });
        if let Some(group_by) = &self.group_by {
            entries = entries.group(
                |e| {
                    let event = &raw.events()[*e];
                    match group_by {
                        GroupBy::Field(name) => event.event.value_by_name(&name).cloned(),
                        GroupBy::Span(selector) => selector.evaluate(raw, event),
                    }
                },
                group_by.clone(),
            );
        }
        entries
    }

    pub(crate) fn load(name: &str) -> Option<Filter> {
        let statements = std::fs::read_to_string(format!("{}.txt", name)).ok()?;
        let mut filter = Filter::default();
        filter.name = name.to_string();
        for line in statements.split("\n") {
            if line.len() == 0 {
                break;
            }
            match line.parse().ok()? {
                Command::GroupBy(group_by) => filter.group_by = Some(group_by),
                Command::Modifier(modifier) => filter.insert_modifier(modifier),
                _ => None?,
            }
        }
        Some(filter)
    }

    pub(crate) fn save(&self, name: &str) -> Option<()> {
        let buffer = self.to_string();
        std::fs::write(format!("{}.txt", name), buffer).ok()
    }
}

impl Display for Filter {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(group_by) = &self.group_by {
            write!(f, "{}\n", group_by)?;
        }
        for modifier in self.modifier.values() {
            write!(f, "{}\n", modifier)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum GroupBy {
    Field(String),
    Span(SpanSelector),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SpanCriterion {
    Field(String),
    Id,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SpanSelector {
    SpanCriterion(SpanCriterion),
    ParentByName {
        name: String,
        criterion: SpanCriterion,
    },
}

impl SpanSelector {
    fn evaluate(&self, store: &Store, entry: &EventEntry) -> Option<Value> {
        let span_id = entry.span?;
        let span = &store.spans()[span_id.0];
        assert_eq!(span.id, span_id);
        match self {
            SpanSelector::ParentByName { name, criterion } => {
                fn check_parent(
                    store: &Store,
                    span: &Span,
                    name: &String,
                    criterion: &SpanCriterion,
                ) -> Option<Value> {
                    let parent = &store.spans()[span.parent_id?.0];
                    if &parent.span.attributes.as_ref()?.metadata.as_ref()?.name == name {
                        match criterion {
                            SpanCriterion::Field(field_name) => {
                                parent.value_by_name(field_name).cloned()
                            }
                            SpanCriterion::Id => Some(Value::from_u64(parent.id.0 as u64)),
                        }
                    } else {
                        check_parent(store, parent, name, criterion)
                    }
                }
                check_parent(store, span, name, criterion)
            }
            SpanSelector::SpanCriterion(criterion) => match criterion {
                SpanCriterion::Field(field_name) => span.value_by_name(field_name).cloned(),
                SpanCriterion::Id => Some(Value::from_u64(span.id.0 as u64)),
            },
        }
    }
}

impl Display for SpanCriterion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SpanCriterion::Field(name) => write!(f, "field.{}", name),
            SpanCriterion::Id => write!(f, "id"),
        }
    }
}

impl Display for SpanSelector {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SpanSelector::SpanCriterion(crit) => write!(f, "{}", crit),
            SpanSelector::ParentByName { name, criterion } => {
                write!(f, r#"parent_by_name("{}").{}"#, name, criterion)
            }
        }
    }
}

impl Display for GroupBy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GroupBy::Field(name) => write!(f, "event.group_by.field.{}", name),
            GroupBy::Span(selector) => write!(f, "event.group_by.span.{}", selector),
        }
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
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
    use super::Modifier;

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
