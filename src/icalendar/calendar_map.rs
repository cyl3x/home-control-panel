use std::collections::{BTreeMap, BTreeSet};
use std::ops::Deref;

use chrono::NaiveDate;
use uuid::Uuid;

use super::{Calendar, Event};

pub type CalendarMap = BTreeMap<Uuid, (Calendar, BTreeMap<Uuid, Event>)>;
pub type CalendarFlatRef<'a> = (&'a Uuid, &'a Calendar, &'a Uuid, &'a Event);

#[derive(Debug, Clone)]
pub enum CalendarMapChange {
  Added(Event),
  Changed(Event),
  Removed(Event),
}

impl Deref for CalendarMapChange {
  type Target = Event;

  fn deref(&self) -> &Self::Target {
    match self {
      Self::Added(t) => t,
      Self::Changed(t) => t,
      Self::Removed(t) => t,
    }
  }
}

impl CalendarMapChange {
  pub fn into_inner(self) -> Event {
    match self {
      Self::Added(t) => t,
      Self::Changed(t) => t,
      Self::Removed(t) => t,
    }
  }

  pub const fn is_removed(&self) -> bool {
    matches!(self, Self::Removed(_))
  }
}

pub trait CalendarMapExt {
  fn flat_iter(&self) -> impl Iterator<Item = CalendarFlatRef>;
  fn flat_events(&self) -> impl Iterator<Item = &Event>;
  fn flat_calendars(&self) -> impl Iterator<Item = &Calendar>;
  fn flat_into_iter(self) -> impl Iterator<Item = (Uuid, Uuid, Event)>;
  fn flat_contains_key(&self, cal_uid: &Uuid, event_uid: &Uuid) -> bool;
  fn flat_get(&self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<&Event>;
  fn flat_get_mut(&mut self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<&mut Event>;
  fn flat_get_key_value(&self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<(&Uuid, &Event)>;
  fn flat_insert(&mut self, cal_uid: &Uuid, event_uid: Uuid, nested: Event) -> Option<Event>;
  fn flat_remove(&mut self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<Event>;
  fn flat_remove_entry(&mut self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<(Uuid, Event)>;
  fn flat_keys(&self) -> BTreeSet<(&Uuid, &Uuid)>;
  fn flat_pop_first(&mut self) -> Option<(Uuid, Uuid, Event)>;
  fn exchange(&mut self, other: Self) -> impl Iterator<Item = (Uuid, Uuid, CalendarMapChange)>;
}

impl CalendarMapExt for CalendarMap {
  fn flat_iter(&self) -> impl Iterator<Item = CalendarFlatRef> {
    self.iter()
      .flat_map(|(cal_uid, (calendar, events))| {
        events
          .iter()
          .map(move |(event_uid, event)| (cal_uid, calendar, event_uid, event))
      })
  }

  fn flat_events(&self) -> impl Iterator<Item = &Event> {
    self.iter()
      .flat_map(|(_, (_, events))| {
        events.iter().map(move |(_, event)| event)
      })
  }

  fn flat_calendars(&self) -> impl Iterator<Item = &Calendar> {
    self.iter().map(|(_, (calendar, _))| calendar)
  }

  fn flat_into_iter(self) -> impl Iterator<Item = (Uuid, Uuid, Event)> {
    self.into_iter()
      .flat_map(|(cal_uid, (_, events))| {
        events
          .into_iter()
          .map(move |(event_uid, event)| (cal_uid, event_uid, event))
      })
  }

  fn flat_contains_key(&self, cal_uid: &Uuid, event_uid: &Uuid) -> bool {
    self.get(cal_uid).map_or(false, |(_, events)| events.contains_key(event_uid))
  }

  fn flat_get(&self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<&Event> {
    self.get(cal_uid).and_then(|(_, events)| events.get(event_uid))
  }

  fn flat_get_mut(&mut self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<&mut Event> {
    self.get_mut(cal_uid).and_then(|(_, events)| events.get_mut(event_uid))
  }

  fn flat_get_key_value(&self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<(&Uuid, &Event)> {
    self.get(cal_uid).and_then(|(_, events)| events.get_key_value(event_uid))
  }

  /// Panics if `cal_uid` does not exist
  fn flat_insert(&mut self, cal_uid: &Uuid, event_uid: Uuid, nested: Event) -> Option<Event> {
    self.get_mut(cal_uid)
      .unwrap()
      .1
      .insert(event_uid, nested)
  }

  fn flat_remove(&mut self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<Event> {
    self.get_mut(cal_uid).and_then(|(_, events)| events.remove(event_uid))
  }

  fn flat_remove_entry(&mut self, cal_uid: &Uuid, event_uid: &Uuid) -> Option<(Uuid, Event)> {
    self.get_mut(cal_uid).and_then(|(_, events)| events.remove_entry(event_uid))
  }

  fn flat_keys(&self) -> BTreeSet<(&Uuid, &Uuid)> {
    self.flat_iter().map(|(cal_uid, _, event_uid, _)| (cal_uid, event_uid)).collect()
  }

  fn flat_pop_first(&mut self) -> Option<(Uuid, Uuid, Event)> {
    if let Some((cal_uid, (calendar, mut events))) = self.pop_first() {
      if let Some((event_uid, event)) = events.pop_first() {
        if !events.is_empty() {
          self.insert(cal_uid, (calendar, events));
        }

        return Some((cal_uid, event_uid, event));
      }
    }

    None
  }

  fn exchange(&mut self, other: Self) -> impl Iterator<Item = (Uuid, Uuid, CalendarMapChange)> {
    let mut other = std::mem::replace(self, other);

    let mut self_iter = self.flat_iter();

    std::iter::from_fn(move || {
      if let Some((first_uid, _, second_uid, nested)) = self_iter.next() {
        if let Some(old_nested) = other.flat_remove(first_uid, second_uid) {
          if &old_nested != nested {
            return Some((*first_uid, *second_uid, CalendarMapChange::Changed(nested.clone())));
          }
        } else {
          return Some((*first_uid, *second_uid, CalendarMapChange::Added(nested.clone())));
        }
      } else if let Some((first_uid, second_uid, old_nested)) = other.flat_pop_first() {
        return Some((first_uid, second_uid, CalendarMapChange::Removed(old_nested)));
      }

      None
    })
  }
}

pub fn all_between<'a>((start, end): (NaiveDate, NaiveDate)) -> impl FnMut(&CalendarFlatRef<'a>) -> bool {
  move |(_, _, _, event)| {
    let event_start = event.start_date();
    let event_end = event.end_date();

    event_end >= start && event_start < end
  }
}
pub fn all_between_events((start, end): (NaiveDate, NaiveDate)) -> impl FnMut(&&Event) -> bool {
  move |event| {
    let event_start = event.start_date();
    let event_end = event.end_date();

    event_end >= start && event_start < end
  }
}