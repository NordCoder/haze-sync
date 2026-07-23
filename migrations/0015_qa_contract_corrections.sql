-- QA contract corrections for complete recovery evidence.
--
-- The reviewed 0012-0014 candidate could create immutable quiescence evidence
-- without the recovery-specific writer facts required by the merged Stage 10
-- recovery contract. Existing incomplete evidence cannot be upgraded truthfully,
-- so this forward migration fails closed if such rows already exist.

do $$
begin
    if exists (select 1 from quiescence_evidence limit 1) then
        raise exception 'incomplete quiescence evidence must be discarded before contract upgrade'
            using errcode = '23514';
    end if;
end
$$;

alter table quiescence_evidence
    add column active_cli_write_jobs bigint not null
        check (active_cli_write_jobs = 0),
    add column object_store_writer_count bigint not null
        check (object_store_writer_count = 0),
    add column database_writer_count bigint not null
        check (database_writer_count = 0),
    add column worktree_external_writer_evidence jsonb not null
        check (
            jsonb_typeof(worktree_external_writer_evidence) in ('object', 'array')
            and worktree_external_writer_evidence <> '{}'::jsonb
            and worktree_external_writer_evidence <> '[]'::jsonb
        );

comment on column quiescence_evidence.active_cli_write_jobs is
    'Captured authoritative CLI writer count; canonical quiescence requires zero.';
comment on column quiescence_evidence.object_store_writer_count is
    'Captured object-store writer count; canonical quiescence requires zero.';
comment on column quiescence_evidence.database_writer_count is
    'Captured database writer count; canonical quiescence requires zero.';
comment on column quiescence_evidence.worktree_external_writer_evidence is
    'Bounded safe evidence proving the captured Worktree path has no uncontrolled writer.';
