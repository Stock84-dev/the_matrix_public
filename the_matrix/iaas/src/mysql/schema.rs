table! {
    exchanges (id) {
        id -> Unsigned<Smallint>,
        name -> Varchar,
        use_testnet -> Bool,
        use_public_data_miner -> Bool,
        api_key -> Varchar,
        api_secret -> Varchar,
        max_leverage -> Float,
        max_orders_per_m -> Float,
    }
}

table! {
    exchange_snapshots (timestamp_ns, exchange_id) {
        timestamp_ns -> Unsigned<Bigint>,
        exchange_id -> Unsigned<Smallint>,
        balance -> Float,
        leverage -> Float,
    }
}

table! {
    funding_snapshots (timestamp_ns, exchange_id, market_model_id) {
        timestamp_ns -> Unsigned<Bigint>,
        market_model_id -> Unsigned<Integer>,
        exchange_id -> Unsigned<Smallint>,
        balance -> Float,
    }
}

table! {
    maintenances (timestamp_s) {
        timestamp_s -> Unsigned<Integer>,
        exchange_id -> Unsigned<Smallint>,
        mode -> Unsigned<Tinyint>,
    }
}

table! {
    market_models (id, exchange_id, model_values_id, market) {
        id -> Unsigned<Integer>,
        exchange_id -> Unsigned<Smallint>,
        model_values_id -> Unsigned<Integer>,
        market -> Varchar,
        target_leverage -> Float,
    }
}

table! {
    models_source (id) {
        id -> Unsigned<Smallint>,
        name -> Varchar,
        source -> Mediumblob,
    }
}

table! {
    models_values (id) {
        id -> Unsigned<Integer>,
        model_source_id -> Unsigned<Smallint>,
        variable_values -> Varbinary,
    }
}

table! {
    position_close_snapshots (timestamp_ns, exchange_id, market_model_id) {
        timestamp_ns -> Unsigned<Bigint>,
        market_model_id -> Unsigned<Integer>,
        exchange_id -> Unsigned<Smallint>,
        balance -> Float,
        expected_amount -> Float,
        actual_amount -> Float,
        expected_price -> Float,
        rounded_price -> Float,
        actual_price -> Float,
    }
}

table! {
    position_open_snapshots (timestamp_ns, exchange_id, market_model_id) {
        timestamp_ns -> Unsigned<Bigint>,
        market_model_id -> Unsigned<Integer>,
        exchange_id -> Unsigned<Smallint>,
        balance -> Float,
        expected_amount -> Float,
        actual_amount -> Float,
        expected_price -> Float,
        rounded_price -> Float,
        actual_price -> Float,
    }
}

joinable!(exchange_snapshots -> exchanges (exchange_id));
joinable!(funding_snapshots -> exchanges (exchange_id));
joinable!(maintenances -> exchanges (exchange_id));
joinable!(market_models -> exchanges (exchange_id));
joinable!(market_models -> models_values (model_values_id));
joinable!(models_values -> models_source (model_source_id));
joinable!(position_close_snapshots -> exchanges (exchange_id));
joinable!(position_open_snapshots -> exchanges (exchange_id));

allow_tables_to_appear_in_same_query!(
    exchanges,
    exchange_snapshots,
    funding_snapshots,
    maintenances,
    market_models,
    models_source,
    models_values,
    position_close_snapshots,
    position_open_snapshots,
);
