use std::ops::Not;

use serde::{Deserialize, Serialize};

use super::{ElementState, VariantStyle};

// just A && B && C for now
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClassCondition<Class> {
    Has(Class),
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
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
    #[serde(bound(serialize = "<T::State as ElementState>::Class: Serialize, T: Serialize"))]
    #[serde(bound(
        deserialize = "<T::State as ElementState>::Class: Deserialize<'de>, T: Deserialize<'de>"
    ))]
    pub Vec<(ClassCondition<<T::State as ElementState>::Class>, T)>,
);

impl<T: VariantStyle> ClassRules<T> {
    pub fn get(&self, variant: &T::State) -> T {
        let mut r = T::default();
        for (condition, item) in &self.0 {
            if condition.eval(variant) {
                r.apply(item);
            }
        }
        r
    }
}
