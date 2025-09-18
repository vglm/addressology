DROP INDEX IF EXISTS fancy_factory_idx;
DROP INDEX IF EXISTS fancy_score_idx;
DROP INDEX IF EXISTS public_key_base_idx;
DROP INDEX IF EXISTS miner_prov_node_id_idx;

CREATE INDEX fancy_factory_idx ON fancy (factory);
CREATE INDEX fancy_score_idx ON fancy (score);
CREATE INDEX public_key_base_idx ON public_key_base (hex);
CREATE INDEX miner_prov_node_id_idx ON miner_info (prov_node_id);


