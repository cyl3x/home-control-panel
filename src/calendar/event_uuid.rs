use std::fmt::{Display, Formatter, Result};

use uuid::Uuid;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct EventUuid(Uuid, Option<usize>);

impl EventUuid {
  pub const fn new(uuid: Uuid, idx: Option<usize>) -> Self {
    Self(uuid, idx)
  }
}

impl PartialOrd for EventUuid {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for EventUuid {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.0.cmp(&other.0).then(self.1.cmp(&other.1))
  }
}

impl Display for EventUuid {
  fn fmt(&self, formatter: &mut Formatter) -> Result {
    formatter.write_fmt(format_args!("{}", self.0))?;

    if let Some(idx) = self.1 {
      return formatter.write_fmt(format_args!("{}-{}", self.0, idx));
    }

    formatter.write_fmt(format_args!("{}", self.0))
  }
}
