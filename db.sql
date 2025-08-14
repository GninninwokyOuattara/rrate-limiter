CREATE TYPE algorithm_type AS ENUM ('fw', 'swc', 'swl', 'tb', 'lb');
CREATE TYPE tracking_type AS ENUM ('ip', 'header');

CREATE TABLE rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    route TEXT NOT NULL UNIQUE,
    expiration INT NOT NULL,
    "limit" INT NOT NULL,
    date_creation TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    date_modification TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    algorithm algorithm_type NOT NULL,
    tracking_type tracking_type NOT NULL,
    custom_tracking_key TEXT,
    status BOOL NOT NULL DEFAULT TRUE,
    ttl INT NOT NULL DEFAULT 60, -- the time to live of the rule itself in the cache
    CONSTRAINT chk_custom_tracking_key CHECK (
        (tracking_type = 'header' AND custom_tracking_key IS NOT NULL) OR
        (tracking_type = 'ip' AND custom_tracking_key IS NULL)
    )
);

CREATE INDEX idx_rules_route ON rules(route);