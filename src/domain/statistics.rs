#[cfg(feature = "statistics")]
pub use inner::*;
#[cfg(feature = "statistics")]
mod inner {
    use crate::{
        bisq::BisqHash,
        domain::{CommandResult, FutureCommandResult},
        prelude::*,
    };
    use actix_web::{web, Error, HttpResponse};
    use iso4217::CurrencyCode;
    use juniper::{
        self, graphql_object,
        http::{graphiql::graphiql_source, GraphQLRequest},
        EmptyMutation, FieldResult, GraphQLInputObject, RootNode,
    };
    use juniper_from_schema::graphql_schema_from_file;
    use std::sync::Arc;

    pub fn graphql(
        schema: web::Data<Arc<Schema>>,
        cache: web::Data<StatsCache>,
        request: web::Json<GraphQLRequest>,
    ) -> impl Future<Item = HttpResponse, Error = Error> {
        web::block(move || {
            let res = request.execute(&schema, &cache);
            Ok::<_, serde_json::error::Error>(serde_json::to_string(&res)?)
        })
        .map_err(Error::from)
        .and_then(|user| {
            Ok(HttpResponse::Ok()
                .content_type("application/json")
                .body(user))
        })
    }
    pub fn graphiql() -> HttpResponse {
        let html = graphiql_source("http://localhost:7477/graphql");
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    }

    graphql_schema_from_file!("src/domain/schema.graphql", context_type: StatsCache);

    pub struct Trade {
        pub currency: CurrencyCode,
        pub hash: BisqHash,
    }

    impl TradeFields for Trade {
        fn field_currency(
            &self,
            executor: &juniper::Executor<'_, StatsCache>,
        ) -> FieldResult<String> {
            Ok(self.currency.alpha3.to_owned())
        }
    }

    pub struct Query;
    impl QueryFields for Query {
        fn field_trades(
            &self,
            executor: &juniper::Executor<'_, StatsCache>,
            trail: &QueryTrail<'_, Trade, juniper_from_schema::Walked>,
        ) -> FieldResult<Vec<&Trade>> {
            Ok(Vec::new())
        }
    }

    type Mutation = EmptyMutation<StatsCache>;

    pub fn create_schema() -> Schema {
        Schema::new(Query {}, EmptyMutation::new())
    }

    type StatsLogInner = Arc<locks::RwLock<Vec<Trade>>>;
    #[derive(Clone)]
    pub struct StatsCache {
        statistics: StatsLogInner,
    }
    impl juniper::Context for StatsCache {}

    impl StatsCache {
        pub fn new() -> Option<Self> {
            Some(Self {
                statistics: Arc::new(locks::RwLock::new(Vec::new())),
            })
        }

        pub fn add(&self, trade: Trade) -> impl FutureCommandResult {
            self.statistics
                .write()
                .map(move |mut guard| {
                    guard.push(trade);
                    CommandResult::Accepted
                })
                .map_err(|_| MailboxError::Closed)
        }
    }
}

#[cfg(not(feature = "statistics"))]
pub use empty::*;
#[cfg(not(feature = "statistics"))]
mod empty {
    use crate::prelude::*;
    use actix_web::{Error, HttpResponse};

    #[derive(Clone)]
    pub struct StatsCache;
    impl StatsCache {
        pub fn new() -> Option<Self> {
            None
        }
    }
    pub struct Schema;
    pub fn create_schema() -> Schema {
        Schema
    }
    pub fn graphql() -> impl Future<Item = HttpResponse, Error = Error> {
        future::ok(HttpResponse::Ok().finish())
    }
    pub fn graphiql() -> HttpResponse {
        HttpResponse::Ok().finish()
    }
}
