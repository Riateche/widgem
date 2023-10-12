use std::{fmt::Display, ops::Not, str::FromStr};

use anyhow::bail;
use itertools::Itertools;
use serde::{de::Error, Deserialize, Serialize};

use super::{Class, ElementState, VariantStyle};

// just A && B && C for now
#[derive(Debug, Clone)]
pub enum ClassCondition<Class> {
    Has(Class),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
}

impl<Class> ClassCondition<Class> {
    fn display_subexpr(&self) -> String
    where
        Class: Display,
    {
        match self {
            ClassCondition::And(c) | ClassCondition::Or(c) if c.len() > 1 => {
                format!("({})", self)
            }
            _ => self.to_string(),
        }
    }
}

impl<Class: Display> Display for ClassCondition<Class> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClassCondition::Has(c) => write!(f, "{c}"),
            ClassCondition::Not(c) => write!(f, "not {}", c.display_subexpr()),
            ClassCondition::And(c) => {
                if c.is_empty() {
                    write!(f, "always")
                } else {
                    write!(f, "{}", c.iter().map(|c| c.display_subexpr()).join(" and "))
                }
            }
            ClassCondition::Or(c) => {
                if c.is_empty() {
                    write!(f, "never")
                } else {
                    write!(f, "{}", c.iter().map(|c| c.display_subexpr()).join(" or "))
                }
            }
        }
    }
}

impl<Class: super::Class> FromStr for ClassCondition<Class> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let words = s.split_ascii_whitespace().collect_vec();
        if words.len() == 3 {
            if words[1] == "and" {
                let a = Class::from_str(words[0])?;
                let b = Class::from_str(words[2])?;
                Ok(Self::has(a).and(b))
            } else if words[2] == "or" {
                let a = Class::from_str(words[0])?;
                let b = Class::from_str(words[2])?;
                Ok(Self::has(a).or(b))
            } else {
                bail!("unrecognized condition: {s:?}");
            }
        } else if words.len() == 1 {
            if words[0] == "always" {
                Ok(Self::always())
            } else {
                let a = Class::from_str(words[0])?;
                Ok(Self::has(a))
            }
        } else if words.len() == 2 && words[0] == "not" {
            let a = Class::from_str(words[1])?;
            Ok(Self::not(a))
        } else {
            bail!("unrecognized condition: {s:?}");
        }
    }
}

impl<Class: Display> Serialize for ClassCondition<Class> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de, Class: super::Class> Deserialize<'de> for ClassCondition<Class> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let str = <String>::deserialize(deserializer)?;
        str.parse().map_err(D::Error::custom)
    }
}

impl<Class> From<Class> for ClassCondition<Class> {
    fn from(value: Class) -> Self {
        ClassCondition::Has(value)
    }
}

impl<Class> ClassCondition<Class> {
    pub fn has(class: Class) -> Self {
        Self::Has(class)
    }

    pub fn not(condition: impl Into<Self>) -> Self {
        Self::Not(Box::new(condition.into()))
    }

    pub fn and(&self, condition: impl Into<Self>) -> Self
    where
        Class: Clone,
    {
        // TODO: simplify multiple ands into one
        Self::And(vec![self.clone(), condition.into()])
    }

    pub fn or(&self, condition: impl Into<Self>) -> Self
    where
        Class: Clone,
    {
        // TODO: simplify multiple ands into one
        Self::Or(vec![self.clone(), condition.into()])
    }

    pub fn always() -> Self {
        Self::And(Vec::new())
    }

    pub fn eval<S>(&self, state: &S) -> bool
    where
        S: ElementState<Class = Class>,
    {
        match self {
            ClassCondition::Has(class) => state.matches(class),
            ClassCondition::Not(condition) => !condition.eval(state),
            ClassCondition::And(conditions) => conditions.iter().all(|c| c.eval(state)),
            ClassCondition::Or(conditions) => conditions.iter().any(|c| c.eval(state)),
        }
    }
}

impl<Class> Not for ClassCondition<Class> {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::Not(Box::new(self))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassRules<T: VariantStyle>(
    #[serde(bound(serialize = "<T::State as ElementState>::Class: Display, T: Serialize"))]
    #[serde(bound(deserialize = "<T::State as ElementState>::Class: Class, T: Deserialize<'de>"))]
    pub Vec<ClassRule<<T::State as ElementState>::Class, T>>,
);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "Class: Display, T: Serialize"))]
#[serde(bound(deserialize = "Class: super::Class, T: Deserialize<'de>"))]
pub struct ClassRule<Class, T> {
    #[serde(rename = "if")]
    pub condition: ClassCondition<Class>,
    #[serde(rename = "then")]
    pub value: T,
}

impl<T: VariantStyle> ClassRules<T> {
    pub fn get(&self, variant: &T::State) -> T {
        let mut r = T::default();
        for rule in &self.0 {
            if rule.condition.eval(variant) {
                r.apply(&rule.value);
            }
        }
        r
    }
}
