-- Independent observation counter; rollback retains non-reuse evidence.
CREATE SEQUENCE plat3_closure_observation_revision AS bigint MINVALUE 1 NO CYCLE;
