SELECT 
    account_platforms.*
FROM account_platforms
INNER JOIN accounts ON account_platforms.account = accounts.id AND accounts.deleted_at = 0
WHERE accounts.id = ?
AND account_platforms.deleted_at = 0
AND account_platforms.platform = ?
ORDER BY account_platforms.created_at ASC