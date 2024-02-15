SELECT
    accounts.*
FROM accounts AS accounts
WHERE accounts.token = ?
AND accounts.token_secret = ?
LIMIT 1