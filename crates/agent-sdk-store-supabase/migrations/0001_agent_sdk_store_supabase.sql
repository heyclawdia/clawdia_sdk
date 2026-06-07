create table if not exists agent_sdk_journal_records (
  store_scope text not null,
  run_id text not null,
  journal_seq bigint not null,
  record jsonb not null,
  primary key (store_scope, run_id, journal_seq)
);

create table if not exists agent_sdk_provider_arguments (
  store_scope text not null,
  provider_ref text not null,
  call_id text not null,
  canonical_tool_name text not null,
  content_ref text not null,
  raw_arguments text not null,
  sha256 text not null,
  primary key (store_scope, provider_ref, call_id)
);

create table if not exists agent_sdk_checkpoints (
  store_scope text not null,
  run_id text not null,
  checkpoint_id text not null,
  covers_journal_seq bigint not null,
  checkpoint jsonb not null,
  primary key (store_scope, run_id, checkpoint_id)
);

create table if not exists agent_sdk_content (
  store_scope text not null,
  content_id text not null,
  content_ref jsonb not null,
  bytes_base64 text not null,
  byte_len bigint not null,
  primary key (store_scope, content_id)
);

create table if not exists agent_sdk_event_archive (
  store_scope text not null,
  position text not null,
  frame jsonb not null,
  primary key (store_scope, position)
);

create table if not exists agent_sdk_agent_pools (
  store_scope text not null,
  pool_id text not null,
  state jsonb not null,
  primary key (store_scope, pool_id)
);

create or replace function agent_sdk_append_journal_record(
  p_store_scope text,
  p_run_id text,
  p_journal_seq bigint,
  p_record jsonb
) returns void
language sql
as $$
  insert into agent_sdk_journal_records (store_scope, run_id, journal_seq, record)
  values (p_store_scope, p_run_id, p_journal_seq, p_record)
  on conflict (store_scope, run_id, journal_seq) do nothing;
$$;

create or replace function agent_sdk_upsert_agent_pool_state(
  p_store_scope text,
  p_pool_id text,
  p_state jsonb
) returns void
language sql
as $$
  insert into agent_sdk_agent_pools (store_scope, pool_id, state)
  values (p_store_scope, p_pool_id, p_state)
  on conflict (store_scope, pool_id)
  do update set state = excluded.state;
$$;

create or replace function agent_sdk_next_agent_pool_event_sequence(
  p_store_scope text,
  p_pool_id text
) returns table(next_sequence bigint)
language plpgsql
as $$
declare
  current_state jsonb;
  next_value bigint;
begin
  select state into current_state
  from agent_sdk_agent_pools
  where store_scope = p_store_scope and pool_id = p_pool_id
  for update;

  if current_state is null then
    raise exception 'agent pool is not open';
  end if;

  next_value := coalesce((current_state ->> 'next_event_counter')::bigint, 0) + 1;
  current_state := jsonb_set(current_state, '{next_event_counter}', to_jsonb(next_value), true);

  update agent_sdk_agent_pools
  set state = current_state
  where store_scope = p_store_scope and pool_id = p_pool_id;

  return query select next_value;
end;
$$;

create or replace function agent_sdk_prune_checkpoints(
  p_store_scope text,
  p_run_id text,
  p_prune_covered_before bigint,
  p_preserve_latest_terminal boolean
) returns void
language sql
as $$
  delete from agent_sdk_checkpoints
  where store_scope = p_store_scope
    and run_id = p_run_id
    and covers_journal_seq < p_prune_covered_before;
$$;
