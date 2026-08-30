-- pg_money 0.2.0 -> 0.3.0
-- Add rusty-money-compatible strict comparison and direct FastMoney helpers.

CREATE FUNCTION money_compare(
    "left" money_with_currency,
    "right" money_with_currency
) RETURNS integer
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'money_compare_wrapper';

CREATE FUNCTION money_minor_compare(
    "left" money_minor,
    "right" money_minor
) RETURNS integer
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'money_minor_compare_wrapper';

CREATE FUNCTION money_minor_from_major(
    major_units bigint,
    currency text
) RETURNS money_minor
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'money_minor_from_major_wrapper';

CREATE FUNCTION money_minor_to_rusty_json(
    value money_minor
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'money_minor_to_rusty_json_wrapper';

CREATE FUNCTION money_minor_from_rusty_json(
    input jsonb
) RETURNS money_minor
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'money_minor_from_rusty_json_wrapper';
