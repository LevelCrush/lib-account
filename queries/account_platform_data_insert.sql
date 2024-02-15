INSERT INTO account_platform_data 
(`account`, `platform`, `key`, `value`, `created_at`, `updated_at`, `deleted_at`)
VALUES {}
ON CONFLICT(`account`,`platform`,`key`)
DO UPDATE SET 
    `value` = excluded.`value`,
    `updated_at` = excluded.`created_at`,
    `deleted_at` = excluded.`deleted_at`