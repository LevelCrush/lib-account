WITH
source_platform AS (
    SELECT
        account_platforms.*
    FROM account_platform_data
    INNER JOIN account_platforms ON
        account_platform_data.platform = account_platforms.id AND
        account_platforms.platform = 'bungie'
    WHERE account_platform_data.key = 'unique_name'
    AND account_platform_data.value = ?
    ORDER BY account_platforms.updated_at DESC
    LIMIT 1
),
discord_data AS (
    SELECT
        source_platform.account,
        discord_platforms.id AS platform,
        displayname_data.value AS display_name,
        username_data.value AS username
    FROM source_platform
    INNER JOIN account_platforms AS discord_platforms ON
        source_platform.account = discord_platforms.account  AND
        discord_platforms.platform = 'discord'
    INNER JOIN account_platform_data AS displayname_data ON
        discord_platforms.id = displayname_data.platform AND
        displayname_data.key = 'display_name'
    INNER JOIN account_platform_data AS username_data ON
        discord_platforms.id = username_data.platform AND
        username_data.key = 'username'
),
bungie_data AS (
    SELECT
        source_platform.account AS account,
        membership_data.platform AS platform,
        membership_data.value AS display_name
    FROM source_platform
    INNER JOIN account_platform_data AS membership_data ON
        source_platform.id = membership_data.platform AND
        source_platform.account = membership_data.account AND
        membership_data.key = 'unique_name'
),
twitch_data AS (
    SELECT
        account_platforms.account AS account,
        membership_data.platform AS platform,
        membership_data.value AS display_name
    FROM source_platform
    INNER JOIN account_platforms ON
        source_platform.account = account_platforms.account AND
        account_platforms.platform = 'twitch'
    INNER JOIN account_platform_data AS membership_data ON
        account_platforms.id = membership_data.platform AND
        account_platforms.account = membership_data.account AND
        membership_data.key = 'display_name'
)
SELECT
    accounts.token AS account_token,
    discord_data.username AS username,
    discord_data.display_name AS discord,
    bungie_data.display_name AS bungie,
    COALESCE(twitch_data.display_name, '') AS twitch
FROM source_platform
INNER JOIN accounts ON source_platform.account = accounts.id
INNER JOIN bungie_data ON
    accounts.id = bungie_data.account AND
    source_platform.id = bungie_data.platform
INNER JOIN discord_data ON accounts.id = discord_data.account
LEFT JOIN twitch_data ON accounts.id = twitch_data.account