use serde::{Deserialize, Serialize};

/// Captured by the caller; matching these values does not authenticate the caller.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationDescriptor {
    pub(crate) version: String,
    pub(crate) package_version: String,
    pub(crate) command: String,
    pub(crate) flags: Vec<String>,
    pub(crate) native_schema: String,
    pub(crate) request_id: String,
    pub(crate) run_id: String,
    pub(crate) process_exit: u64,
}
impl InvocationDescriptor {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() > 16 * 1024 {
            return Err("invocation exceeds limit");
        }
        let value =
            codeguard_adapters::parse_unique_json(bytes).map_err(|_| "invalid invocation JSON")?;
        let invocation: Self =
            serde_json::from_value(value).map_err(|_| "invalid invocation fields")?;
        invocation.validate()?;
        Ok(invocation)
    }
    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        let invocation = self;
        if invocation.version != "codeguard.invocation/v1alpha1"
            || invocation.package_version != "0.1.4"
            || invocation.command != "lint"
            || invocation.flags != ["--format=json"]
            || invocation.native_schema != "1.0"
            || invocation.request_id.trim().is_empty()
            || invocation.run_id.trim().is_empty()
            || !matches!(invocation.process_exit, 0 | 1 | 3 | 4 | 130)
        {
            return Err("unsupported invocation profile");
        }
        Ok(())
    }
}

/// Informational capability declaration; no caller declaration can qualify a reader.
#[derive(Debug, Clone, Serialize)]
pub struct CapabilityProfile {
    pub version: &'static str,
    pub native_schema: &'static str,
    pub qualified: bool,
    pub mapping_version: Option<&'static str>,
    pub engine_wire_version: Option<&'static str>,
}
impl InvocationDescriptor {
    /// Returns the locally registered capability, never caller-supplied qualification.
    pub fn capability(&self) -> Result<CapabilityProfile, &'static str> {
        self.validate()?;
        Ok(CapabilityProfile {
            version: "codeguard.capability/v1alpha1",
            native_schema: "1.0",
            qualified: false,
            mapping_version: None,
            engine_wire_version: None,
        })
    }
}
