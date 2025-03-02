use nu_plugin::LabeledError;
use nu_protocol::{ast::CellPath, Span, Value};
use semver::{BuildMetadata, Prerelease, Version};

#[derive(Debug, Eq, PartialEq)]
pub enum Action {
    SemVerAction(SemVerAction),
    Default,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SemVerAction {
    Major,
    Minor,
    Patch,
}

#[derive(Default)]
pub struct Inc {
    pub error: Option<String>,
    pub cell_path: Option<CellPath>,
    pub action: Option<Action>,
}

impl Inc {
    pub fn new() -> Self {
        Default::default()
    }

    fn apply(&self, input: &str, head: Span) -> Value {
        match &self.action {
            Some(Action::SemVerAction(act_on)) => {
                let mut ver = match semver::Version::parse(input) {
                    Ok(parsed_ver) => parsed_ver,
                    Err(_) => return Value::string(input, head),
                };

                match act_on {
                    SemVerAction::Major => Self::increment_major(&mut ver),
                    SemVerAction::Minor => Self::increment_minor(&mut ver),
                    SemVerAction::Patch => Self::increment_patch(&mut ver),
                }

                Value::string(ver.to_string(), head)
            }
            Some(Action::Default) | None => {
                if let Ok(v) = input.parse::<u64>() {
                    Value::string((v + 1).to_string(), head)
                } else {
                    Value::string(input, head)
                }
            }
        }
    }

    pub fn increment_patch(v: &mut Version) {
        v.patch += 1;
        v.pre = Prerelease::EMPTY;
        v.build = BuildMetadata::EMPTY;
    }

    pub fn increment_minor(v: &mut Version) {
        v.minor += 1;
        v.patch = 0;
        v.pre = Prerelease::EMPTY;
        v.build = BuildMetadata::EMPTY;
    }

    pub fn increment_major(v: &mut Version) {
        v.major += 1;
        v.minor = 0;
        v.patch = 0;
        v.pre = Prerelease::EMPTY;
        v.build = BuildMetadata::EMPTY;
    }

    pub fn for_semver(&mut self, part: SemVerAction) {
        if self.permit() {
            self.action = Some(Action::SemVerAction(part));
        } else {
            self.log_error("can only apply one");
        }
    }

    fn permit(&mut self) -> bool {
        self.action.is_none()
    }

    fn log_error(&mut self, message: &str) {
        self.error = Some(message.to_string());
    }

    pub fn usage() -> &'static str {
        "Usage: inc field [--major|--minor|--patch]"
    }

    pub fn inc(&self, head: Span, value: &Value) -> Result<Value, LabeledError> {
        if let Some(cell_path) = &self.cell_path {
            let working_value = value.clone();
            let cell_value = working_value.follow_cell_path(&cell_path.members, false)?;

            let cell_value = self.inc_value(head, &cell_value)?;

            let mut value = value.clone();
            value
                .update_data_at_cell_path(&cell_path.members, cell_value)
                .map_err(|x| {
                    let error: LabeledError = x.into();
                    error
                })?;
            Ok(value)
        } else {
            self.inc_value(head, value)
        }
    }

    pub fn inc_value(&self, head: Span, value: &Value) -> Result<Value, LabeledError> {
        match value {
            Value::Int { val, .. } => Ok(Value::int(val + 1, head)),
            Value::String { val, .. } => Ok(self.apply(val, head)),
            x => {
                let msg = x.as_string().map_err(|e| LabeledError {
                    label: "Unable to extract string".into(),
                    msg: format!("value cannot be converted to string {x:?} - {e}"),
                    span: Some(head),
                })?;

                Err(LabeledError {
                    label: "Incorrect value".into(),
                    msg,
                    span: Some(head),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    mod semver {
        use nu_protocol::{Span, Value};

        use crate::inc::SemVerAction;
        use crate::Inc;

        #[test]
        fn major() {
            let expected = Value::test_string("1.0.0");
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Major);
            assert_eq!(inc.apply("0.1.3", Span::test_data()), expected)
        }

        #[test]
        fn minor() {
            let expected = Value::test_string("0.2.0");
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Minor);
            assert_eq!(inc.apply("0.1.3", Span::test_data()), expected)
        }

        #[test]
        fn patch() {
            let expected = Value::test_string("0.1.4");
            let mut inc = Inc::new();
            inc.for_semver(SemVerAction::Patch);
            assert_eq!(inc.apply("0.1.3", Span::test_data()), expected)
        }
    }
}
