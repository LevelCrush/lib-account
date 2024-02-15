SELECT
    accounts.*
FROM accounts
WHERE accounts.id = ?
AND accounts.deleted_at = 0
LIMIT 1