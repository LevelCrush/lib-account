SELECT 
    accounts.*
FROM account_platforms
INNER JOIN accounts ON account_platforms.account = accounts.id AND accounts.deleted_at = 0
WHERE account_platforms.platform = ?
AND account_platforms.platform_user = ?