#![allow(unused)]

mod boredapi {
    use std::str::FromStr;
    use strum_macros;
    use std::{fmt, collections, marker};
    use std::borrow::Borrow;
    use std::cmp;
    use url;
    use std::marker::PhantomData;

    /// Represents a type of activity in Bored API.
    #[derive(strum_macros::EnumString, strum_macros::ToString, cmp::PartialEq, cmp::Eq, fmt::Debug)]
    pub enum ActivityType {
        #[strum(serialize = "education")]
        Education,
        #[strum(serialize = "recreational")]
        Recreational,
        #[strum(serialize = "social")]
        Social,
        #[strum(serialize = "diy")]
        Diy,
        #[strum(serialize = "charity")]
        Charity,
        #[strum(serialize = "cooking")]
        Cooking,
        #[strum(serialize = "relaxation")]
        Relaxation,
        #[strum(serialize = "music")]
        Music,
        #[strum(serialize = "busywork")]
        Busywork,
    }

    /// Combines all possible errors of the API wrapper.
    #[derive(fmt::Debug)]
    pub enum Error {
        /// Error returned by reqwest.
        HttpError(reqwest::Error),
        /// Error returned by API.
        ApiError(String),
        /// Error caused by a bad read of API response. Possible problems are invalid Bored API
        /// backend or bug in the wrapper.
        BadResponse,
    }

    /// Represents Activity entity of Bored API.
    #[derive(fmt::Debug)]
    pub struct Activity {
        pub description: String,
        pub accessibility: f64,
        pub activity_type: ActivityType,
        pub participants: u64,
        pub price: f64,
        pub link: Option<url::Url>,
        pub key: u64,
        dummy: PhantomData<()>,
    }

    impl Activity {
        pub fn new(description: String,
                   accessibility: f64,
                   activity_type: ActivityType,
                   participants: u64,
                   price: f64,
                   link: Option<url::Url>,
                   key: u64) -> Self {
            Activity { description, accessibility, activity_type, participants, price, link, key, dummy: PhantomData {} }
        }
    }

    #[derive(fmt::Debug)]
    pub struct ActivityCriterion<T> {
        name: &'static str,
        validate: fn(T) -> bool,
    }

    pub const EXACT_ACCESSIBILITY: ActivityCriterion<f64> = ActivityCriterion {
        name: "accessibility",
        validate: |v| (0.0..1.0).contains(&v),
    };

    pub const EXACT_PRICE: ActivityCriterion<f64> = ActivityCriterion {
        name: "price",
        validate: |v| (0.0..1.0).contains(&v),
    };

    pub const KEY: ActivityCriterion<u64> = ActivityCriterion {
        name: "key",
        validate: |v| (1000000..9999999).contains(&v),
    };

    pub const MAX_ACCESSIBILITY: ActivityCriterion<f64> = ActivityCriterion {
        name: "maxaccessibility",
        validate: |v| (0.0..1.0).contains(&v),
    };

    pub const MAX_PRICE: ActivityCriterion<f64> = ActivityCriterion {
        name: "maxprice",
        validate: |v| (0.0..1.0).contains(&v),
    };

    pub const MIN_ACCESSIBILITY: ActivityCriterion<f64> = ActivityCriterion {
        name: "minaccessibility",
        validate: |v| (0.0..1.0).contains(&v),
    };

    pub const MIN_PRICE: ActivityCriterion<f64> = ActivityCriterion {
        name: "minprice",
        validate: |v| (0.0..1.0).contains(&v),
    };

    pub const PARTICIPANTS: ActivityCriterion<u64> = ActivityCriterion {
        name: "participants",
        validate: |v| (0..u64::MAX).contains(&v),
    };

    pub const TYPE: ActivityCriterion<ActivityType> = ActivityCriterion {
        name: "type",
        validate: |_| true,
    };

    #[derive(fmt::Debug)]
    pub struct CriteriaSelection { parameters: collections::HashMap<String, String> }

    impl CriteriaSelection {
        pub fn set<T: ToString>(mut self, criterion: ActivityCriterion<T>, value: T) -> Self {
            self.parameters.insert(criterion.name.to_string(), value.to_string());
            return self;
        }
    }

    impl Clone for CriteriaSelection {
        fn clone(&self) -> Self {
            CriteriaSelection { parameters: self.parameters.clone() }
        }
    }

    impl Default for CriteriaSelection {
        fn default() -> Self {
            CriteriaSelection { parameters: collections::HashMap::new() }
        }
    }

    #[derive(fmt::Debug)]
    pub struct BoredApi {
        pub url: &'static str,
        pub client: reqwest::Client,
    }

    impl Default for BoredApi {
        fn default() -> Self {
            BoredApi { url: "http://www.boredapi.com/api/activity", client: reqwest::Client::new() }
        }
    }

    impl Clone for BoredApi {
        fn clone(&self) -> Self {
            return BoredApi { url: self.url, client: self.client.clone() };
        }
    }

    impl BoredApi {
        pub async fn random(self) -> Result<Activity, Error> {
            self.by_criteria(|s| s).await
        }

        pub async fn by_criteria<F: FnOnce(CriteriaSelection) -> CriteriaSelection>(self, selection: F) -> Result<Activity, Error> {
            let mut sel = CriteriaSelection::default();
            sel = selection(sel);

            match self.client.get(self.url).query(&sel.parameters.borrow()).send().await {
                Ok(r) => match r.json::<serde_json::Value>().await {
                    Ok(val) => self.deserialize(val),
                    Err(r) => Err(Error::HttpError(r))
                },
                Err(r) => Err(Error::HttpError(r)),
            }
        }

        #[inline]
        fn deserialize(self, json: serde_json::Value) -> Result<Activity, Error> {
            macro_rules! extract_field {
            ($name:expr, $extractor:ident) => {
                json.get($name).ok_or(Error::BadResponse)?.$extractor().ok_or(Error::BadResponse)?
            };
            }

            if let Some(err) = json.get("error") {
                return Err(err
                    .as_str()
                    .map(|s| Error::ApiError(s.to_string()))
                    .unwrap_or(Error::BadResponse));
            }

            Ok(Activity::new(
                extract_field!("activity", as_str).to_string(),
                extract_field!("accessibility", as_f64),
                ActivityType::from_str(extract_field!("type", as_str))
                    .map_err(|_| Error::BadResponse)?,
                extract_field!("participants", as_u64),
                extract_field!("price", as_f64),
                match extract_field!("link", as_str) {
                    "" => None,
                    s => Some(url::Url::parse(s).map_err(|_| Error::BadResponse)?),
                },
                extract_field!("key", as_str).parse::<u64>().map_err(|e| Error::BadResponse)?,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::boredapi;
    use tokio::runtime::Runtime;
    use crate::boredapi::{Error, Activity};

    macro_rules! aw {
    ($e:expr) => {
        Runtime::new().expect("").block_on($e)
    };
  }

    #[test]
    fn random() {
        match aw!(boredapi::BoredApi::default().random()) {
            Ok(a) => { println!("{:?}", a); }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn by_criteria() {
        match aw!(boredapi::BoredApi::default().by_criteria(|sel| sel.set(boredapi::TYPE, boredapi::ActivityType::Busywork))) {
            Ok(a) => {
                assert_eq!(a.activity_type, boredapi::ActivityType::Busywork);
                println!("{:?}", a)
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn no_activity() {
        match aw!(boredapi::BoredApi::default().by_criteria(|s| s.set(boredapi::EXACT_ACCESSIBILITY, -1.0))) {
            Ok(_) => assert!(false),
            Err(e) => match e {
                Error::HttpError(_) => assert!(false),
                Error::ApiError(e) => { assert_eq!(e, "No activity found with the specified parameters") }
                Error::BadResponse => assert!(false),
            },
        }
    }
}
