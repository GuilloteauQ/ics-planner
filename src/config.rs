use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::LinkedList;
use std::collections::{BinaryHeap, HashMap};
use uuid::Uuid;
extern crate ics;

use ics::properties::{DtEnd, DtStart, Status, Summary};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct WorkTask {
    timeslots: usize,
    divisible: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
struct MandatoryEvent {
    #[serde(with = "my_hour_format")]
    start: NaiveTime,
    #[serde(with = "my_hour_format")]
    end: NaiveTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(with = "my_date_format")]
    day: NaiveDate,
    mandatory: HashMap<String, MandatoryEvent>,
    tasks: HashMap<String, WorkTask>,
    #[serde(with = "my_hour_format")]
    start_time: NaiveTime,
    #[serde(with = "my_hour_format")]
    end_time: NaiveTime,
    #[serde(with = "my_hour_format")]
    timeslots_size: NaiveTime,
}

impl Config {
    //     pub fn get_working_day_span(&self) -> Duration {
    //         NaiveTime::signed_duration_since(self.end_time, self.start_time)
    //     }
    //
    //     pub fn get_mandatory_time(&self) -> Duration {
    //         self.mandatory
    //             .iter()
    //             .map(|(_, v)| NaiveTime::signed_duration_since(v.end, v.start))
    //             .fold(Duration::zero(), |d, acc| d + acc)
    //     }
    //
    //     pub fn get_available_time(&self) -> Duration {
    //         self.get_working_day_span() - self.get_mandatory_time()
    //     }
    //
    //     pub fn get_number_available_timeslots(&self) -> i64 {
    //         self.get_available_time().num_seconds()
    //             / NaiveTime::signed_duration_since(self.timeslots_size, NaiveTime::from_hms(0, 0, 0))
    //                 .num_seconds()
    //     }
    //
    fn sort_mandatory_tasks_by_start_time(&self) -> Vec<MandatoryEvent> {
        let mut sorted_tasks: Vec<MandatoryEvent> = self
            .mandatory
            .iter()
            .map(|(_, v)| *v)
            .collect::<Vec<MandatoryEvent>>();
        sorted_tasks.sort_by_key(|x| x.start);
        sorted_tasks
    }

    fn get_work_tasks_not_divisible_queue(&self) -> LinkedList<String> {
        self.tasks.iter().filter(|(_, v)| !v.divisible).fold(
            LinkedList::new(),
            |mut acc, (k, v)| {
                (0..(v.timeslots)).for_each(|_| acc.push_front(k.to_string()));
                acc
            },
        )
    }

    fn get_work_tasks_divisible_queue(&self) -> BinaryHeap<DivisibleTask> {
        let dur =
            NaiveTime::signed_duration_since(self.timeslots_size, NaiveTime::from_hms(0, 0, 0));
        self.tasks
            .iter()
            .filter(|(_, v)| v.divisible)
            .fold(BinaryHeap::new(), |mut acc, (k, v)| {
                (0..(v.timeslots)).for_each(|_| acc.push(DivisibleTask::new(k.to_string(), dur)));
                acc
            })
    }

    fn build_schedule(&self) -> Schedule {
        let mut schedule: Schedule = LinkedList::new();
        let mut current_start_time = self.start_time;
        let mut mandatory_events = self.sort_mandatory_tasks_by_start_time();
        mandatory_events.push(MandatoryEvent {
            start: self.end_time,
            end: self.end_time,
        });
        let mut undivisible_work_tasks = self.get_work_tasks_not_divisible_queue();
        let mut divisible_work_tasks = self.get_work_tasks_divisible_queue();
        let timeslot =
            NaiveTime::signed_duration_since(self.timeslots_size, NaiveTime::from_hms(0, 0, 0));
        for mandatory_event in mandatory_events.iter() {
            let next_mandatory_start_time = mandatory_event.start;

            let mut duration_to_next_mandatory =
                NaiveTime::signed_duration_since(next_mandatory_start_time, current_start_time);
            while !undivisible_work_tasks.is_empty() && duration_to_next_mandatory >= timeslot {
                schedule.push_back(Event::new(
                    undivisible_work_tasks.pop_front().unwrap(),
                    current_start_time,
                    current_start_time + timeslot,
                ));
                current_start_time = current_start_time + timeslot;
                duration_to_next_mandatory =
                    NaiveTime::signed_duration_since(next_mandatory_start_time, current_start_time);
            }
            while !divisible_work_tasks.is_empty() && duration_to_next_mandatory > Duration::zero()
            {
                // this means that there is some room left before the next mandatory event
                let mut divisible_task = divisible_work_tasks.pop().unwrap();

                if divisible_task.dur > duration_to_next_mandatory {
                    schedule.push_back(Event::new(
                        divisible_task.name.clone(),
                        current_start_time,
                        current_start_time + duration_to_next_mandatory,
                    ));
                    divisible_task.dur = divisible_task.dur - duration_to_next_mandatory;
                    divisible_work_tasks.push(divisible_task);
                    current_start_time = mandatory_event.end;
                } else {
                    schedule.push_back(Event::new(
                        divisible_task.name,
                        current_start_time,
                        current_start_time + divisible_task.dur,
                    ));
                    current_start_time = current_start_time + divisible_task.dur;
                }
                duration_to_next_mandatory =
                    NaiveTime::signed_duration_since(next_mandatory_start_time, current_start_time);
            }
            current_start_time = mandatory_event.end;
        }

        schedule
    }

    // pub fn to_ics(&self, filename: String) {
    pub fn to_ics(&self) {
        let schedule = self.build_schedule();
        let mut calendar =
            ics::ICalendar::new("2.0", "-//xyz Corp//NONSGML PDA Calendar Version 1.0//EN");

        for e in schedule.iter() {
            calendar.add_event(e.to_ics(self.day));
        }
        println!("{}", calendar);
        // calendar
        //     .save_file(filename)
        //     .expect("could not write calendar")
    }
}

#[derive(Debug)]
struct Event {
    name: String,
    start_time: NaiveTime,
    end_time: NaiveTime,
}

fn get_formatted_time(date: NaiveDate, time: NaiveTime) -> String {
    format!(
        "{:#02}{:#02}{:#02}T{:#02}{:#02}{:#02}Z",
        date.year(),
        date.month(),
        date.day(),
        time.hour(),
        time.minute(),
        time.second()
    )
}

impl Event {
    fn new(name: String, start_time: NaiveTime, end_time: NaiveTime) -> Self {
        Event {
            name,
            start_time,
            end_time,
        }
    }

    fn to_ics(&self, day: NaiveDate) -> ics::Event {
        let my_uuid = Uuid::new_v4();
        let fmt_start = get_formatted_time(day, self.start_time);
        let fmt_end = get_formatted_time(day, self.end_time);

        let mut event = ics::Event::new(
            my_uuid.to_hyphenated().to_string(),
            get_formatted_time(day, NaiveTime::from_hms(0, 0, 0)),
        );
        event.push(DtStart::new(fmt_start));
        event.push(DtEnd::new(fmt_end));
        event.push(Status::confirmed());
        event.push(Summary::new(self.name.clone()));
        event
    }
}

struct DivisibleTask {
    dur: Duration,
    name: String,
}

impl DivisibleTask {
    fn new(name: String, dur: Duration) -> Self {
        DivisibleTask { name, dur }
    }
}

impl PartialEq for DivisibleTask {
    fn eq(&self, other: &Self) -> bool {
        self.dur == other.dur
    }
}

impl Eq for DivisibleTask {}

impl PartialOrd for DivisibleTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.dur.partial_cmp(&other.dur)
    }
}

impl Ord for DivisibleTask {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dur.cmp(&other.dur)
    }
}

type Schedule = LinkedList<Event>;

mod my_date_format {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d";

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

mod my_hour_format {
    use chrono::NaiveTime;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%H:%M:%S";

    pub fn serialize<S>(date: &NaiveTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveTime::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}
