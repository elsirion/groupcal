use askama::Template;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::cmp::max;
use std::collections::BTreeMap;
use std::io::BufReader;
use std::path::PathBuf;
use structopt::StructOpt;

const COLORS: &[&'static str] = &[
    "#AAE9E5", "#87C7F1", "#FEB7D3", "#FFEDA9", "#EACFFF", "#DEE6C8", "#A8D0C6",
];

#[derive(Debug, StructOpt)]
struct Options {
    cal_file: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Calendar(Vec<Event>);

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Event {
    title: String,
    start: NaiveDate,
    end: NaiveDate,
    certainty: Certainty,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
enum Certainty {
    Sure,
    Possible,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct CalendarCols {
    cols: Vec<BTreeMap<NaiveDate, RowEntry>>,
    last_day: Option<NaiveDate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RowEntry {
    first: bool,
    color: String,
    event: Event,
}

#[derive(Template)]
#[template(path = "index.html")]
struct Index {
    cols: CalendarCols,
}

impl CalendarCols {
    pub fn from_events(mut events: Vec<Event>) -> CalendarCols {
        let colors = COLORS.iter().cycle();

        events.sort_by_key(|event| event.start);

        let mut rows = CalendarCols {
            cols: Default::default(),
            last_day: None,
        };
        for (event, color) in events.into_iter().zip(colors) {
            rows.insert_event_chronological(event, (*color).to_owned());
        }

        rows
    }

    fn insert_event_chronological(&mut self, event: Event, color: String) {
        let row = self.find_free_row(&event.start);
        let day_iter = event.start.iter_days().take_while(|day| day <= &event.end);

        let mut first = true;
        for day in day_iter {
            row.insert(
                day,
                RowEntry {
                    first,
                    color: color.clone(),
                    event: event.clone(),
                },
            );
            first = false;
        }

        self.last_day = Some(
            self.last_day
                .map(|ld| max(ld, event.end))
                .unwrap_or(event.end),
        );
    }

    fn find_free_row(&mut self, date: &NaiveDate) -> &mut BTreeMap<NaiveDate, RowEntry> {
        let row_idx = self.cols.iter().enumerate().find_map(|(idx, events)| {
            if events.contains_key(date) {
                None
            } else {
                Some(idx)
            }
        });
        if let Some(row_idx) = row_idx {
            return &mut self.cols[row_idx];
        } else {
            self.cols.push(Default::default());
            self.cols.last_mut().expect("just created it")
        }
    }

    fn maybe_row_iter<'a>(
        &'a self,
    ) -> Option<impl Iterator<Item = (NaiveDate, usize, Vec<Option<RowEntry>>)> + 'a> {
        let first = self.cols.first()?.iter().next()?.0;
        let last = self.last_day?;

        Some(
            first
                .iter_days()
                .take_while(move |day| day <= &last)
                .map(|day| {
                    let count = self
                        .cols
                        .iter()
                        .filter(|col| col.contains_key(&day))
                        .count();
                    let cells = self.cols.iter().map(|col| col.get(&day).cloned()).collect();
                    (day, count, cells)
                }),
        )
    }

    pub fn rows(&self) -> Vec<(NaiveDate, usize, Vec<Option<RowEntry>>)> {
        self.maybe_row_iter().into_iter().flatten().collect()
    }

    pub fn num_cols(&self) -> usize {
        self.cols.len()
    }
}

fn main() {
    let options: Options = StructOpt::from_args();

    let file = std::fs::File::open(options.cal_file).expect("Couldn't open calendar file");
    let events: Calendar =
        serde_json::from_reader(BufReader::new(file)).expect("Invalid calendar file");
    let cols = CalendarCols::from_events(events.0);

    let template = Index { cols };

    println!("{}", Template::render(&template).unwrap());
}
