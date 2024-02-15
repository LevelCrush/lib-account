SELECT
    account_platforms.platform,
    account_platforms.platform_user,
    account_platform_data.key,
    account_platform_data.value
FROM account_platform_data
INNER JOIN account_platforms ON account_platform_data.platform = account_platforms.id
INNER JOIN accounts ON account_platform_data.account = accounts.id
WHERE account_platform_data.account = ?
ORDER BY account_platforms.platform ASC, account_platforms.id ASC, account_platform_data.key ASC