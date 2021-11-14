use bigdecimal::BigDecimal;

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

pub trait ToClause {
    fn to_clause(&self, field: String) -> QueryClause;
}

#[cfg(test)]
mod tests {

    use bigdecimal::BigDecimal;
    use bigdecimal::FromPrimitive;

    use query_derive::Clauseable;

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
}
