create table if not exists public.inference_requests (
  request_id uuid primary key,
  user_id uuid not null references auth.users(id) on delete cascade,
  request_kind text not null check (request_kind in ('continue')),
  status text not null check (status in ('reserved', 'succeeded', 'failed')),
  input_tokens integer,
  output_tokens integer,
  provider_response_id text,
  latency_ms integer,
  error_code text,
  created_at timestamptz not null default now(),
  completed_at timestamptz
);

create index if not exists inference_requests_user_created_idx
  on public.inference_requests (user_id, created_at desc);

alter table public.inference_requests enable row level security;
revoke all on table public.inference_requests from anon, authenticated;

create or replace function public.reserve_inference_request(
  p_request_id uuid,
  p_request_kind text
)
returns jsonb
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_user_id uuid := auth.uid();
  v_month_count integer;
  v_minute_count integer;
  v_concurrent_count integer;
begin
  if v_user_id is null or coalesce(auth.jwt() ->> 'role', '') <> 'authenticated' then
    raise exception 'authentication_required' using errcode = '42501';
  end if;
  if p_request_kind <> 'continue' then
    return jsonb_build_object('allowed', false, 'code', 'request_kind_not_allowed');
  end if;

  perform pg_advisory_xact_lock(hashtextextended(v_user_id::text, 0));

  if exists (select 1 from public.inference_requests where request_id = p_request_id) then
    return jsonb_build_object('allowed', false, 'code', 'duplicate_request');
  end if;

  select count(*) into v_month_count
  from public.inference_requests
  where user_id = v_user_id
    and request_kind = 'continue'
    and created_at >= date_trunc('month', now());

  select count(*) into v_minute_count
  from public.inference_requests
  where user_id = v_user_id
    and request_kind = 'continue'
    and created_at >= now() - interval '1 minute';

  select count(*) into v_concurrent_count
  from public.inference_requests
  where user_id = v_user_id
    and request_kind = 'continue'
    and status = 'reserved'
    and created_at >= now() - interval '5 minutes';

  if v_month_count >= 100 then
    return jsonb_build_object('allowed', false, 'code', 'monthly_quota_exceeded');
  end if;
  if v_minute_count >= 10 then
    return jsonb_build_object('allowed', false, 'code', 'rate_limit_exceeded', 'retry_after_seconds', 60);
  end if;
  if v_concurrent_count >= 2 then
    return jsonb_build_object('allowed', false, 'code', 'concurrency_limit_exceeded', 'retry_after_seconds', 15);
  end if;

  insert into public.inference_requests (request_id, user_id, request_kind, status)
  values (p_request_id, v_user_id, p_request_kind, 'reserved');

  return jsonb_build_object('allowed', true, 'code', 'reserved');
end;
$$;

create or replace function public.complete_inference_request(
  p_request_id uuid,
  p_status text,
  p_input_tokens integer default null,
  p_output_tokens integer default null,
  p_provider_response_id text default null,
  p_latency_ms integer default null,
  p_error_code text default null
)
returns boolean
language plpgsql
security definer
set search_path = ''
as $$
declare
  v_user_id uuid := auth.uid();
begin
  if v_user_id is null or coalesce(auth.jwt() ->> 'role', '') <> 'authenticated' then
    raise exception 'authentication_required' using errcode = '42501';
  end if;
  if p_status not in ('succeeded', 'failed') then
    raise exception 'invalid_completion_status' using errcode = '22023';
  end if;

  update public.inference_requests
  set status = p_status,
      input_tokens = p_input_tokens,
      output_tokens = p_output_tokens,
      provider_response_id = left(p_provider_response_id, 255),
      latency_ms = greatest(p_latency_ms, 0),
      error_code = left(p_error_code, 120),
      completed_at = now()
  where request_id = p_request_id
    and user_id = v_user_id
    and status = 'reserved';

  return found;
end;
$$;

revoke all on function public.reserve_inference_request(uuid, text) from public, anon;
revoke all on function public.complete_inference_request(uuid, text, integer, integer, text, integer, text) from public, anon;
grant execute on function public.reserve_inference_request(uuid, text) to authenticated;
grant execute on function public.complete_inference_request(uuid, text, integer, integer, text, integer, text) to authenticated;

comment on table public.inference_requests is
  'Operational inference metadata only. Never store evidence, prompts, model output, URLs, paths, titles, or screenshots here.';
