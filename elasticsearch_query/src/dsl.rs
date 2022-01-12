use core::fmt;

use bigdecimal::BigDecimal;
use serde::ser::SerializeMap;
use serde::Serialize;

/// A query clauses which represent an Elasticserach Leaf Query DSL.
/// [Query DSL]: <https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryClause {
    Match {
        field: String,
        search_val: String,
    },
    Range {
        field: String,
        gte: BigDecimal,
        lte: BigDecimal,
    },
    Terms {
        field: String,
        search_val: Vec<String>,
    },
    Prefix {
        field: String,
        search_val: String,
        is_case_insensitive: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySort {
    pub field_name: String,
    pub ordering: SortType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortType {
    Asc,
    Desc,
}

impl fmt::Display for SortType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SortType::Asc => write!(f, "asc"),
            SortType::Desc => write!(f, "desc"),
        }
    }
}

impl Serialize for QuerySort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.field_name, &self.ordering.to_string())?;
        map.end()
    }
}

pub struct InnerQueryClause<'a>(&'a QueryClause);
pub struct InnerRange<'a>(&'a BigDecimal, &'a BigDecimal);

pub struct InnerPrefix<'a>(&'a String, &'a bool);

impl<'a> Serialize for InnerQueryClause<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let query = self.0;
        match query {
            QueryClause::Match { field, search_val } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(field, search_val)?;
                map.end()
            }
            QueryClause::Range { field, gte, lte } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(field, &InnerRange(gte, lte))?;
                map.end()
            }
            QueryClause::Terms { field, search_val } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(field, search_val)?;
                map.end()
            }
            QueryClause::Prefix {
                field,
                search_val,
                is_case_insensitive,
            } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(field, &InnerPrefix(search_val, is_case_insensitive))?;
                map.end()
            }
        }
    }
}

impl<'a> Serialize for InnerRange<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("gte", self.0)?;
        map.serialize_entry("lte", self.1)?;
        map.end()
    }
}

impl<'a> Serialize for InnerPrefix<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("value", self.0)?;
        map.serialize_entry("case_insensitive", self.1)?;
        map.end()
    }
}

impl Serialize for QueryClause {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            q @ QueryClause::Match { .. } => map.serialize_entry("match", &InnerQueryClause(q))?,
            q @ QueryClause::Range { .. } => map.serialize_entry("range", &InnerQueryClause(q))?,
            q @ QueryClause::Terms { .. } => map.serialize_entry("terms", &InnerQueryClause(q))?,
            q @ QueryClause::Prefix { .. } => {
                map.serialize_entry("prefix", &InnerQueryClause(q))?
            }
        }
        map.end()
    }
}

pub trait ToClause {
    fn to_clause(&self, field: String) -> QueryClause;
}

#[cfg(test)]
mod tests {

    use bigdecimal::{BigDecimal, FromPrimitive};

    use elasticsearch_query_derive::Clauseable;
    use serde_json::json;

    use super::*;

    use super::ToClause;

    #[derive(Debug, Clone, Copy)]
    pub struct Range {
        lower_bound: Option<i32>,
        upper_bound: Option<i32>,
    }

    impl ToClause for Range {
        fn to_clause(&self, field: String) -> QueryClause {
            let range = self;
            let gte = match range.lower_bound {
                Some(lower_bound) => BigDecimal::from_i32(lower_bound).unwrap(),
                None => BigDecimal::from_i32(i32::MIN).unwrap(),
            };

            let lte = match range.upper_bound {
                Some(upper_bound) => BigDecimal::from_i32(upper_bound).unwrap(),
                None => BigDecimal::from_i32(i32::MAX).unwrap(),
            };

            QueryClause::Range { field, gte, lte }
        }
    }

    #[derive(Clauseable)]
    pub struct SearchCriteria {
        #[search_field("risk_spectrum")]
        pub fund_info_risk_spectrum: Option<Range>,
        #[search_field("fund_statistics.return_ytd")]
        pub fund_statistics_return_ytd: Option<Range>,
    }

    #[test]
    fn given_non_none_range_to_clauses_should_return_correct_vec_of_query_clause() {
        let criteria = SearchCriteria {
            fund_info_risk_spectrum: Some(Range {
                lower_bound: Some(1),
                upper_bound: Some(10),
            }),
            fund_statistics_return_ytd: Some(Range {
                lower_bound: Some(5),
                upper_bound: Some(6),
            }),
        };
        let clauses: Vec<QueryClause> = criteria.to_clauses();
        let expected = vec![
            QueryClause::Range {
                field: "risk_spectrum".into(),
                lte: BigDecimal::from_i32(10).unwrap(),
                gte: BigDecimal::from_i32(1).unwrap(),
            },
            QueryClause::Range {
                field: "fund_statistics.return_ytd".into(),
                lte: BigDecimal::from_i32(6).unwrap(),
                gte: BigDecimal::from_i32(5).unwrap(),
            },
        ];
        assert_eq!(expected, clauses,)
    }

    #[test]
    fn given_one_none_range_to_clauses_should_return_correct_vec_of_query_clause() {
        let criteria = SearchCriteria {
            fund_info_risk_spectrum: None,
            fund_statistics_return_ytd: Some(Range {
                lower_bound: Some(5),
                upper_bound: Some(6),
            }),
        };
        let clauses: Vec<QueryClause> = criteria.to_clauses();
        let expected = vec![QueryClause::Range {
            field: "fund_statistics.return_ytd".into(),
            lte: BigDecimal::from_i32(6).unwrap(),
            gte: BigDecimal::from_i32(5).unwrap(),
        }];
        assert_eq!(expected, clauses,)
    }

    #[test]
    fn query_match_clause_should_serialize_correctly() {
        let expect = json!({
            "match": {
                "fund_name": "global"
            }
        })
        .to_string();
        let query = QueryClause::Match {
            field: "fund_name".into(),
            search_val: "global".into(),
        };

        assert_eq!(expect, json!(query).to_string());
    }

    #[test]
    fn query_range_clause_should_serialize_correctly() {
        let expect = json!({
            "range": {
                "risk_spectrum": {
                    "gte": "2",
                    "lte": "5"
                }
            }
        })
        .to_string();
        let query = QueryClause::Range {
            field: "risk_spectrum".into(),
            gte: BigDecimal::from_i32(2).unwrap(),
            lte: BigDecimal::from_i32(5).unwrap(),
        };

        assert_eq!(expect, json!(query).to_string());
    }

    #[test]
    fn query_sort_should_serialize_correctly() {
        let expect = json!({
            "risk_spectrum": "asc"
        })
        .to_string();
        let sort = QuerySort {
            field_name: "risk_spectrum".into(),
            ordering: SortType::Asc,
        };

        assert_eq!(expect, json!(sort).to_string());

        let expect = json!({
            "risk_spectrum": "desc"
        })
        .to_string();
        let sort = QuerySort {
            field_name: "risk_spectrum".into(),
            ordering: SortType::Desc,
        };

        assert_eq!(expect, json!(sort).to_string());
    }

    #[test]
    fn query_terms_clause_should_serialize_correctly() {
        let expect = json!({
          "terms": {
            "fund_id": ["1", "2", "4"]
          }
        })
        .to_string();
        let query = QueryClause::Terms {
            field: "fund_id".into(),
            search_val: vec!["1".to_string(), "2".to_string(), "4".to_string()],
        };

        assert_eq!(expect, json!(query).to_string());
    }

    #[test]
    fn query_prefix_clause_should_serialize_correctly() {
        let expect = json!({
          "prefix": {
            "fund_code" : {
                "value": "k-ghealth",
                "case_insensitive": true
            }
          }
        })
        .to_string();
        let query = QueryClause::Prefix {
            field: "fund_code".into(),
            search_val: "k-ghealth".to_string(),
            is_case_insensitive: true,
        };

        assert_eq!(expect, json!(query).to_string());
    }
}
