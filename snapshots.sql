CREATE TABLE models_source(
	id SMALLINT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
	name VARCHAR(255) NOT NULL,
	source MEDIUMBLOB NOT NULL
);

CREATE TABLE models_values(
	id INT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
	model_source_id SMALLINT UNSIGNED NOT NULL,
	variable_values VARBINARY(255) NOT NULL,
	FOREIGN KEY(model_source_id) REFERENCES models_source(id)
);

CREATE TABLE exchanges(
	id SMALLINT UNSIGNED PRIMARY KEY AUTO_INCREMENT,
	name VARCHAR(255) NOT NULL,
    use_testnet BOOLEAN NOT NULL,
    use_public_data_miner BOOLEAN NOT NULL,
    api_key VARCHAR(255) NOT NULL,
    api_secret VARCHAR(255) NOT NULL,
    max_leverage FLOAT NOT NULL,
    max_orders_per_m FLOAT NOT NULL
);

CREATE TABLE market_models(
	id INT UNSIGNED AUTO_INCREMENT,
	exchange_id SMALLINT UNSIGNED NOT NULL,
	model_values_id INT UNSIGNED NOT NULL,
	market VARCHAR(255) NOT NULL,
	target_leverage FLOAT NOT NULL,
	FOREIGN KEY(exchange_id) REFERENCES exchanges(id),
	FOREIGN KEY(model_values_id) REFERENCES models_values(id),
	PRIMARY KEY (id, exchange_id, model_values_id, market)
);

CREATE TABLE maintenances(
	timestamp_s INT UNSIGNED,
	exchange_id SMALLINT UNSIGNED NOT NULL,
	mode TINYINT UNSIGNED NOT NULL,
	FOREIGN KEY(exchange_id) REFERENCES exchanges(id),
	PRIMARY KEY (timestamp_s)
);

CREATE TABLE funding_snapshots(
	timestamp_ns BIGINT UNSIGNED,
	market_model_id INT UNSIGNED,
	exchange_id SMALLINT UNSIGNED,
	balance FLOAT NOT NULL,
	FOREIGN KEY(market_model_id) REFERENCES market_models(id),
	FOREIGN KEY(exchange_id) REFERENCES exchanges(id),
	PRIMARY KEY (timestamp_ns, exchange_id, market_model_id)
);

CREATE TABLE position_open_snapshots(
	timestamp_ns BIGINT UNSIGNED,
	market_model_id INT UNSIGNED,
	exchange_id SMALLINT UNSIGNED,
	balance FLOAT NOT NULL,
	expected_amount FLOAT NOT NULL,
	actual_amount FLOAT NOT NULL,
	expected_price FLOAT NOT NULL,
	rounded_price FLOAT NOT NULL,
	actual_price FLOAT NOT NULL,
	FOREIGN KEY(market_model_id) REFERENCES market_models(id),
	FOREIGN KEY(exchange_id) REFERENCES exchanges(id),
	PRIMARY KEY (timestamp_ns, exchange_id, market_model_id)
);

CREATE TABLE position_close_snapshots(
	timestamp_ns BIGINT UNSIGNED,
	market_model_id INT UNSIGNED,
	exchange_id SMALLINT UNSIGNED,
	balance FLOAT NOT NULL,
	expected_amount FLOAT NOT NULL,
	actual_amount FLOAT NOT NULL,
	expected_price FLOAT NOT NULL,
	rounded_price FLOAT NOT NULL,
	actual_price FLOAT NOT NULL,
	FOREIGN KEY(market_model_id) REFERENCES market_models(id),
	FOREIGN KEY(exchange_id) REFERENCES exchanges(id),
	PRIMARY KEY (timestamp_ns, exchange_id, market_model_id)
);

CREATE TABLE exchange_snapshots(
	timestamp_ns BIGINT UNSIGNED NOT NULL,
	exchange_id SMALLINT UNSIGNED NOT NULL,
	balance FLOAT NOT NULL,
	leverage FLOAT NOT NULL,
	FOREIGN KEY(exchange_id) REFERENCES exchanges(id),
	PRIMARY KEY (timestamp_ns, exchange_id)
);

INSERT INTO exchanges (name,use_testnet,use_public_data_miner,api_key,api_secret,max_leverage,max_orders_per_m)
VALUES ("BitMEX Testnet", true, true, "api key", "api secret", 1, 2);

INSERT INTO market_models (exchange_id ,model_values_id, market, target_leverage)
VALUES(1, 1, "ETHUSD", 1);
