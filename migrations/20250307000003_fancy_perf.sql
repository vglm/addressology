DROP INDEX IF EXISTS fancy_factory_idx;
DROP INDEX IF EXISTS fancy_score_idx;
DROP INDEX IF EXISTS public_key_base_idx;
DROP INDEX IF EXISTS job_info_miner_idx;

CREATE INDEX fancy_factory_idx ON fancy (factory);
CREATE INDEX fancy_score_idx ON fancy (score);
CREATE INDEX public_key_base_idx ON public_key_base (hex);
CREATE INDEX job_info_miner_idx ON job_info (miner);


