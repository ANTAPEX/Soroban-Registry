-- Add upgrade strategy enum and column to contracts

CREATE TYPE upgrade_strategy_type AS ENUM ('proxy', 'uups', 'data_migration', 'shadow_contract');

ALTER TABLE contracts
    ADD COLUMN upgrade_strategy upgrade_strategy_type NULL;

CREATE INDEX idx_contracts_upgrade_strategy ON contracts(upgrade_strategy);
