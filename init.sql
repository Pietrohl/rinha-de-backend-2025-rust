GRANT ALL PRIVILEGES ON DATABASE rinha_2025_db TO postgres;

CREATE UNLOGGED TABLE transactions (
    correlation_id uuid DEFAULT gen_random_uuid(),
    processed_at timestamp DEFAULT CURRENT_TIMESTAMP NOT NULL,
    amount integer NOT NULL,
    service varchar(10) NOT NULL,
    PRIMARY KEY (correlation_id)
);