SELECT 
    account_platforms.*
FROM account_platforms
WHERE account_platforms.platform = ?
AND account_platforms.platform_user = ?
ORDER BY account_platforms.created_at ASC
LIMIT 1