SELECT
    account_platforms.platform_user AS discord_id
FROM account_platforms
WHERE account_platforms.platform = ?
ORDER BY account_platforms.updated_at ASC
LIMIT ?