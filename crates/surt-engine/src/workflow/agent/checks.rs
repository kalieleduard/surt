use super::{AgentContract, Auth, Format, OutputSpec, Profile, RuntimeKind};
use crate::workflow::error::Problem;
use crate::workflow::files::SchemaSource;

const SCHEMA_VERSION: i64 = 1;
const TEST_RUNTIMES_ALLOWED: bool = cfg!(any(test, feature = "test-support"));

impl AgentContract {
    pub(super) fn check(&self, id: &str, schemas: &impl SchemaSource) -> Result<(), Problem> {
        if self.schema != SCHEMA_VERSION {
            return Err(Problem::UnsupportedSchema { found: self.schema });
        }
        if self.id != id {
            return Err(Problem::NameMismatch {
                found: self.id.clone(),
            });
        }
        self.check_formats()?;
        self.check_output_outcomes()?;
        self.check_deliveries()?;
        self.profile.check()?;
        match self
            .schema_paths()
            .find(|path| schemas.schema_text(path).is_none())
        {
            Some(path) => Err(Problem::MissingAsset {
                path: path.to_owned(),
            }),
            None => Ok(()),
        }
    }

    fn check_formats(&self) -> Result<(), Problem> {
        let inputs = self
            .inputs
            .iter()
            .map(|(name, input)| (name, input.format, &input.json_schema));
        let outputs = self
            .outputs
            .iter()
            .map(|(name, output)| (name, output.format, &output.json_schema));
        if let Some((name, ..)) = inputs
            .chain(outputs)
            .find(|(_, format, schema)| schema.is_some() && !accepts_json_schema(*format))
        {
            return Err(Problem::MisplacedJsonSchema { name: name.clone() });
        }
        self.outputs
            .iter()
            .try_for_each(|(name, output)| output.check_item_files(name))
    }
}

impl OutputSpec {
    fn check_item_files(&self, name: &str) -> Result<(), Problem> {
        if !self.item_files.is_empty() && self.format != Format::ItemList {
            return Err(Problem::MisplacedItemFiles {
                name: name.to_owned(),
            });
        }
        let invalid = self.item_files.iter().find(|(_, file)| {
            file.format == Format::ItemList
                || (file.json_schema.is_some() && file.format != Format::Json)
        });
        match invalid {
            Some((file, _)) => Err(Problem::UnsupportedItemFileFormat { name: file.clone() }),
            None => Ok(()),
        }
    }
}

impl Profile {
    fn check(&self) -> Result<(), Problem> {
        if self.runtime == RuntimeKind::Fake && !TEST_RUNTIMES_ALLOWED {
            return Err(Problem::TestOnlyRuntime);
        }
        if self.runtime == RuntimeKind::ClaudeCode && self.claude.is_none() {
            return Err(Problem::MissingClaudeProfile);
        }
        if self
            .claude
            .as_ref()
            .is_some_and(|claude| claude.api_key_env.is_some() && claude.auth != Auth::ApiKey)
        {
            return Err(Problem::MisplacedApiKeyEnv);
        }
        let budget = self
            .limits
            .as_ref()
            .and_then(|limits| limits.max_budget_usd);
        if budget.is_some_and(|usd| !(usd.is_finite() && usd > 0.0)) {
            return Err(Problem::InvalidBudget);
        }
        Ok(())
    }
}

fn accepts_json_schema(format: Format) -> bool {
    matches!(format, Format::Json | Format::ItemList)
}
