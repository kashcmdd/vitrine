use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Newtype identifiers.
///
/// UUIDv7 rather than v4: the high bits are a millisecond timestamp, so primary
/// keys are roughly insertion-ordered and index locality is good without adding
/// a separate sort column.
macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Generates a fresh time-ordered id.
            pub fn generate() -> Self {
                Self(Uuid::now_v7().to_string())
            }

            /// Wraps an id that came from the database. Does not validate shape:
            /// rows are trusted, request payloads are not.
            pub fn from_db(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                // Rejects garbage at the IPC boundary before it reaches SQL.
                Uuid::parse_str(value)?;
                Ok(Self(value.to_owned()))
            }
        }
    };
}

id_type!(
    /// A library entry: a PC game, or an emulated title when `system_id` is set.
    GameId
);
id_type!(
    /// A console platform, carrying its packaging geometry.
    SystemId
);
id_type!(
    /// A compatibility layer or emulator.
    RunnerId
);
id_type!(
    /// One preset of launch arguments and wrappers for a game.
    LaunchConfigId
);
id_type!(
    /// One contiguous period during which a game was running.
    SessionId
);
id_type!(
    /// Reserved for a future multi-profile feature; not yet referenced by schema.
    PlayerId
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_ids_are_time_ordered() {
        let first = GameId::generate();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = GameId::generate();
        assert!(first < second, "uuid v7 should sort by creation time");
    }

    #[test]
    fn parsing_rejects_non_uuids() {
        assert!("not-an-id".parse::<GameId>().is_err());
        assert!("../etc/passwd".parse::<GameId>().is_err());
    }

    #[test]
    fn parsing_accepts_uuids() {
        let id = GameId::generate();
        let round_tripped: GameId = id.as_str().parse().expect("valid uuid");
        assert_eq!(id, round_tripped);
    }
}
