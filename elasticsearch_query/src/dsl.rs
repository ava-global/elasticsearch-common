use bigdecimal::BigDecimal;
use serde::ser::SerializeMap;
use serde::Serialize;

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
}

pub struct InnerQueryClause<'a>(&'a QueryClause);
pub struct InnerRange<'a>(&'a BigDecimal, &'a BigDecimal);

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

impl Serialize for QueryClause {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            q @ QueryClause::Match { .. } => map.serialize_entry("match", &InnerQueryClause(q))?,
            q @ QueryClause::Range { .. } => map.serialize_entry("range", &InnerQueryClause(q))?,
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

    use crate::dsl::QueryClause;

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
}
