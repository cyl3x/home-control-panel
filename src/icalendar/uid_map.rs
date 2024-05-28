use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

use uuid::Uuid;

pub type UuidMap<T> = BTreeMap<Uuid, T>;
pub type UuidMapRef<'a, T> = BTreeMap<&'a Uuid, T>;
pub type UuidNestedMap<T> = UuidMap<UuidMap<T>>;
pub type UuidNestedVecRef<'a, T> = Vec<(&'a Uuid, &'a Uuid, &'a T)>;
pub type UidChangeset<'a, T> = UidMap<UidMapChange<'a, T>>;

#[derive(Debug)]
pub enum UidMapChange<'a, T> {
  Added(&'a T),
  Changed(&'a T),
  Removed(T),
}

impl<'a, T> Deref for UidMapChange<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    match self {
      UidMapChange::Added(t) => t,
      UidMapChange::Changed(t) => t,
      UidMapChange::Removed(t) => t,
    }
  }
}

#[derive(Debug, PartialEq, Eq, Default, PartialOrd, Ord)]
pub struct UidMap<V>(UuidNestedMap<V>);

impl<T> UidMap<T> {
  pub const fn new() -> Self {
    Self(BTreeMap::new())
  }

  pub fn flat_iter(&self) -> impl Iterator<Item = (&Uuid, &Uuid, &T)> {
    self.0.iter()
      .flat_map(|(first_uid, nest)| {
          nest
            .iter()
            .map(move |(second_uid, nested)| (first_uid, second_uid, nested))
      })
  }

  pub fn flat_into_iter(self) -> impl Iterator<Item = (Uuid, Uuid, T)> {
    self.0.into_iter()
      .flat_map(|(first_uid, nest)| {
          nest
            .into_iter()
            .map(move |(second_uid, nested)| (first_uid, second_uid, nested))
      })
  }

  pub fn flat_contains_key(&self, first_uid: &Uuid, second_uid: &Uuid) -> bool {
    self.0.get(first_uid).map_or(false, |nest| nest.contains_key(second_uid))
  }

  pub fn flat_get(&self, first_uid: &Uuid, second_uid: &Uuid) -> Option<&T> {
    self.0.get(first_uid).and_then(|nest| nest.get(second_uid))
  }

  pub fn flat_get_mut(&mut self, first_uid: &Uuid, second_uid: &Uuid) -> Option<&mut T> {
    self.0.get_mut(first_uid).and_then(|nest| nest.get_mut(second_uid))
  }

  pub fn flat_get_key_value(&self, first_uid: &Uuid, second_uid: &Uuid) -> Option<(&Uuid, &T)> {
    self.0.get(first_uid).and_then(|nest| nest.get_key_value(second_uid))
  }

  pub fn flat_entry(&mut self, first_uid: Uuid, second_uid: Uuid) -> std::collections::btree_map::Entry<Uuid, T> {
    self.0.entry(first_uid).or_default().entry(second_uid)
  }

  pub fn flat_insert(&mut self, first_uid: Uuid, second_uid: Uuid, nested: T) -> Option<T> {
    self.0.entry(first_uid)
      .or_default()
      .insert(second_uid, nested)
  }

  pub fn flat_remove(&mut self, first_uid: &Uuid, second_uid: &Uuid) -> Option<T> {
    self.0.get_mut(first_uid).and_then(|nest| nest.remove(second_uid))
  }

  pub fn flat_remove_entry(&mut self, first_uid: &Uuid, second_uid: &Uuid) -> Option<(Uuid, T)> {
    self.0.get_mut(first_uid).and_then(|nest| nest.remove_entry(second_uid))
  }

  pub fn flat_keys(&self) -> BTreeSet<(&Uuid, &Uuid)> {
    self.flat_iter().map(|(first_uid, second_uid, _)| (first_uid, second_uid)).collect()
  }

  pub fn to_vec(&self) -> UuidNestedVecRef<T> {
    self.flat_iter().collect()
  }

  /// Only merges on first level and will override
  pub fn merge(&mut self, mut other: Self) {
    let map = std::mem::take(&mut other.0);

    map
      .into_iter()
      .for_each(|(first_uid, nested)| {
        self.insert(first_uid, nested);
      });
  }
}

impl<T> std::ops::Deref for UidMap<T> {
  type Target = UuidNestedMap<T>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl<T> std::ops::DerefMut for UidMap<T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

impl<T> From<UuidNestedMap<T>> for UidMap<T> {
  fn from(map: UuidNestedMap<T>) -> Self {
    Self(map)
  }
}

impl<T> UidMap<T> where T: PartialEq + Clone {
  // /// Returns a changeset of two UidMaps, where the other is considered the new state
  // pub fn get_changeset<'a>(&'a self, other: &'a UidMap<T>) -> UidChangeset<'a, T> {
  //   let mut changeset = UidMap::new();

  //   for (first_uid, nest) in other.iter() {
  //     for (second_uid, nested) in nest.iter() {
  //       if let Some(old_nested) = self.flat_get(first_uid, second_uid) {
  //         if old_nested != nested {
  //           changeset.flat_insert(*first_uid, *second_uid, UidMapChange::Changed(nested));
  //         }
  //       } else {
  //         changeset.flat_insert(*first_uid, *second_uid, UidMapChange::Added(nested));
  //       }
  //     }
  //   }

  //   for (first_uid, nest) in self.iter() {
  //     for (second_uid, nested) in nest.iter() {
  //       if !other.flat_contains_key(first_uid, second_uid) {
  //         changeset.flat_insert(*first_uid, *second_uid, UidMapChange::Removed(nested));
  //       }
  //     }
  //   }

  //   changeset
  // }

  // pub fn apply_changeset(&mut self, changeset: &UidChangeset<T>) {
  //   for (first_uid, second_uid, change) in changeset.flat_iter() {
  //     match change {
  //       UidMapChange::Added(nested) => {
  //         self.flat_insert(*first_uid, *second_uid, (*nested).clone());
  //       },
  //       UidMapChange::Changed(nested) => {
  //         self.flat_insert(*first_uid, *second_uid, (*nested).clone());
  //       },
  //       UidMapChange::Removed(_) => {
  //         self.flat_remove(first_uid, second_uid);
  //       },
  //     }
  //   }
  // }

  pub fn get_applied_changeset(&mut self, mut other: Self) -> UidChangeset<'_, T> {
    let mut changeset = UidMap::new();

    std::mem::swap(&mut self.0, &mut other.0);

    for (first_uid, second_uid, nested) in self.flat_iter() {
      if !other.flat_contains_key(first_uid, second_uid) {
        changeset.flat_insert(*first_uid, *second_uid, UidMapChange::Added(nested));
      } else if other.flat_remove(first_uid, second_uid).unwrap() != *nested {
        changeset.flat_insert(*first_uid, *second_uid, UidMapChange::Changed(nested));
      }
    }

    for (first_uid, second_uid, old_nested) in other.flat_into_iter() {
      changeset.flat_insert(first_uid, second_uid, UidMapChange::Removed(old_nested));
    }

    changeset
  }
}

impl<T> FromIterator<(Uuid, Uuid, T)> for UidMap<T> {
  fn from_iter<I: IntoIterator<Item = (Uuid, Uuid, T)>>(iter: I) -> Self {
    let mut map = Self::new();

    for (first_uid, second_uid, nested) in iter {
      map.flat_insert(first_uid, second_uid, nested);
    }

    map
  }
}
