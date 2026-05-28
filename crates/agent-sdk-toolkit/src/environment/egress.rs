use agent_sdk_core::{AgentError, NetworkIsolationPolicy};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
/// Transport/application hint for an environment egress target.
/// Selecting a protocol is data-only; concrete enforcement belongs to the
/// registered isolation runtime adapter.
pub enum EgressProtocol {
    /// HTTPS-style egress, defaulting to port 443.
    Https,
    /// HTTP-style egress, defaulting to port 80.
    Http,
    /// TCP egress, requiring a concrete port.
    Tcp,
    /// UDP egress, requiring a concrete port.
    Udp,
}

impl EgressProtocol {
    fn from_scheme(scheme: &str) -> Option<Self> {
        match scheme {
            "https" => Some(Self::Https),
            "http" => Some(Self::Http),
            "tcp" => Some(Self::Tcp),
            "udp" => Some(Self::Udp),
            _ => None,
        }
    }

    fn default_port(self) -> Option<u16> {
        match self {
            Self::Https => Some(443),
            Self::Http => Some(80),
            Self::Tcp | Self::Udp => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Https => "https",
            Self::Http => "http",
            Self::Tcp => "tcp",
            Self::Udp => "udp",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
/// One canonical egress target requested for an isolated environment.
/// The value is policy intent, not proof that a runtime has enforced it.
pub struct EgressTarget {
    /// Domain or host alias requested by policy.
    pub host: String,
    /// Destination port requested by policy.
    pub port: u16,
    /// Protocol hint for adapter translation.
    pub protocol: EgressProtocol,
}

impl EgressTarget {
    /// Parses and canonicalizes an egress target.
    ///
    /// Accepted forms include `example.com`, `example.com:443`,
    /// `https://example.com`, `http://example.com:80`,
    /// `tcp://example.com:443`, and `udp://resolver.example:53`.
    /// URL paths, credentials, query strings, fragments, wildcards, empty hosts,
    /// and invalid ports are rejected because this value describes an egress
    /// boundary rather than a fetch request.
    pub fn parse(input: impl AsRef<str>) -> Result<Self, AgentError> {
        let input = input.as_ref().trim();
        if input.is_empty() {
            return Err(invalid_egress_target("egress target is empty"));
        }
        if input.chars().any(char::is_whitespace) {
            return Err(invalid_egress_target(
                "egress target must not contain whitespace",
            ));
        }

        let (protocol, authority) = if let Some((scheme, rest)) = input.split_once("://") {
            let scheme = scheme.to_ascii_lowercase();
            let protocol = EgressProtocol::from_scheme(&scheme).ok_or_else(|| {
                invalid_egress_target(format!("unsupported egress protocol: {scheme}"))
            })?;
            (protocol, rest)
        } else {
            (EgressProtocol::Https, input)
        };

        if authority
            .chars()
            .any(|ch| matches!(ch, '/' | '?' | '#' | '@' | '\\'))
        {
            return Err(invalid_egress_target(
                "egress target must be a host or host:port, not a URL path or credential",
            ));
        }

        let (host, port) = parse_authority(authority, protocol)?;
        validate_host(&host)?;
        Ok(Self {
            host: host.to_ascii_lowercase(),
            port,
            protocol,
        })
    }

    /// Returns the canonical string lowered into `NetworkIsolationPolicy`.
    pub fn canonical(&self) -> String {
        format!("{}://{}:{}", self.protocol.as_str(), self.host, self.port)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Data-only builder for deterministic egress allowlists.
pub struct EgressAllowlist {
    entries: Vec<String>,
}

impl EgressAllowlist {
    /// Creates an empty allowlist.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one allowlist target for validation during lowering.
    pub fn allow(mut self, target: impl Into<String>) -> Self {
        self.entries.push(target.into());
        self
    }

    /// Adds and validates one allowlist target immediately.
    pub fn try_allow(mut self, target: impl AsRef<str>) -> Result<Self, AgentError> {
        self.entries.push(EgressTarget::parse(target)?.canonical());
        Ok(self)
    }

    /// Builds an allowlist from multiple targets.
    pub fn from_targets(targets: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut allowlist = Self::new();
        for target in targets {
            allowlist = allowlist.allow(target);
        }
        allowlist
    }

    /// Builds and validates an allowlist from multiple targets.
    pub fn try_from_targets(
        targets: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<Self, AgentError> {
        let mut allowlist = Self::new();
        for target in targets {
            allowlist = allowlist.try_allow(target)?;
        }
        Ok(allowlist)
    }

    /// Returns canonical targets in deterministic order.
    pub fn targets(&self) -> Result<Vec<EgressTarget>, AgentError> {
        self.parsed_targets()
    }

    /// Returns canonical egress rule entries suitable for core `NetworkIsolationPolicy`.
    pub fn canonical_entries(&self) -> Result<Vec<String>, AgentError> {
        Ok(self
            .parsed_targets()?
            .iter()
            .map(EgressTarget::canonical)
            .collect::<Vec<_>>())
    }

    /// Lowers this toolkit helper into the core network policy contract.
    pub fn network_policy(&self) -> Result<NetworkIsolationPolicy, AgentError> {
        Ok(NetworkIsolationPolicy::EgressScoped {
            rules: self.canonical_entries()?,
        })
    }

    /// Returns whether the allowlist has no targets.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn parsed_targets(&self) -> Result<Vec<EgressTarget>, AgentError> {
        let mut targets = self
            .entries
            .iter()
            .map(EgressTarget::parse)
            .collect::<Result<Vec<_>, _>>()?;
        targets.sort();
        targets.dedup();
        Ok(targets)
    }
}

fn parse_authority(authority: &str, protocol: EgressProtocol) -> Result<(String, u16), AgentError> {
    if authority.is_empty() {
        return Err(invalid_egress_target("egress target host is empty"));
    }
    if authority.matches(':').count() > 1 {
        return Err(invalid_egress_target(
            "egress target must not use raw IPv6 or multiple port separators",
        ));
    }
    if let Some((host, port)) = authority.rsplit_once(':') {
        if host.is_empty() {
            return Err(invalid_egress_target("egress target host is empty"));
        }
        let port = port
            .parse::<u16>()
            .map_err(|_| invalid_egress_target("egress target port must be 1..65535"))?;
        if port == 0 {
            return Err(invalid_egress_target("egress target port must be nonzero"));
        }
        Ok((host.to_string(), port))
    } else if let Some(port) = protocol.default_port() {
        Ok((authority.to_string(), port))
    } else {
        Err(invalid_egress_target(
            "tcp and udp egress targets must include an explicit port",
        ))
    }
}

fn validate_host(host: &str) -> Result<(), AgentError> {
    if host.is_empty() {
        return Err(invalid_egress_target("egress target host is empty"));
    }
    if host == "*" || host.contains('*') {
        return Err(invalid_egress_target(
            "egress target host wildcards are not supported",
        ));
    }
    if host.starts_with('.') || host.ends_with('.') {
        return Err(invalid_egress_target(
            "egress target host must not start or end with '.'",
        ));
    }
    for label in host.split('.') {
        if label.is_empty() {
            return Err(invalid_egress_target(
                "egress target host must not contain empty labels",
            ));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(invalid_egress_target(
                "egress target host labels must not start or end with '-'",
            ));
        }
        if !label
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
        {
            return Err(invalid_egress_target(
                "egress target host labels must be ASCII alphanumeric or '-'",
            ));
        }
    }
    Ok(())
}

fn invalid_egress_target(message: impl Into<String>) -> AgentError {
    AgentError::contract_violation(message.into())
}
