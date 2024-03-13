INSERT INTO account_platform_data 
(`account`, `platform`, `key`, `value`, `created_at`, `updated_at`, `deleted_at`)
VALUES {}
ON DUPLICATE KEY
UPDATE
    `value` = values(`value`),
    `updated_at` = values(`created_at`),
    `deleted_at` = values(`deleted_at`)