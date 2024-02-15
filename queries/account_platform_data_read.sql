SELECT
    account_platform_data.id,
    account_platform_data.key
FROM account_platform_data
INNER JOIN account_platforms ON account_platform_data.platform = account_platforms.id
INNER JOIN accounts ON account_platforms.account = accounts.id AND accounts.deleted_at = 0
WHERE accounts.id = ?
AND account_platforms.id = ?
AND account_platform_data.key IN ({})