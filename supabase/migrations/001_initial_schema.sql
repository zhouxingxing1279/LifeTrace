create extension if not exists "pgcrypto";

create or replace function public.set_updated_at() returns trigger language plpgsql as $$
begin new.updated_at = timezone('utc', now()); return new; end; $$;

create table public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  user_id uuid not null unique references auth.users(id) on delete cascade,
  display_name text, week_starts_on smallint not null default 1 check (week_starts_on between 0 and 6),
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now())
);
create table public.activities (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade,
  name text not null, type text not null check(type in ('duration','count','completion','weekly','control')), unit text not null,
  minimum_target numeric, normal_target numeric, target_period text not null check(target_period in ('daily','weekly')),
  target_days smallint[], icon text, description text, is_archived boolean not null default false,
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), deleted_at timestamptz
);
create table public.activity_plans (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade, activity_id uuid not null references public.activities(id) on delete cascade,
  plan_date date, week_start date, target_value numeric, status text not null default 'active',
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), deleted_at timestamptz
);
create table public.activity_logs (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade, activity_id uuid not null references public.activities(id) on delete cascade,
  started_at timestamptz, ended_at timestamptz, value numeric, status text check(status in ('completed','partial','skipped')), note text, mood smallint check(mood between 1 and 10), metadata jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), deleted_at timestamptz
);
create table public.daily_reviews (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade, review_date date not null,
  energy smallint not null check(energy between 1 and 10), mood smallint not null check(mood between 1 and 10), completion_score numeric,
  best_thing text, problem text, tomorrow_priority text, note text,
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), unique(user_id, review_date)
);
create table public.finance_accounts (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade, name text not null,
  type text not null check(type in ('cash','bank','wechat','alipay','other')), initial_balance_cents bigint not null default 0, is_archived boolean not null default false,
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), deleted_at timestamptz
);
create table public.transaction_categories (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade, name text not null,
  type text not null check(type in ('expense','income')), icon text, is_system boolean not null default false,
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), deleted_at timestamptz
);
create table public.transactions (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade,
  account_id uuid not null references public.finance_accounts(id), type text not null check(type in ('expense','income','transfer')),
  amount_cents bigint not null check(amount_cents > 0), category_id uuid references public.transaction_categories(id), occurred_at timestamptz not null, note text,
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), deleted_at timestamptz
);
create table public.sync_events (
  id uuid primary key default gen_random_uuid(), user_id uuid not null references auth.users(id) on delete cascade,
  entity_type text not null, entity_id uuid not null, operation text not null check(operation in ('upsert','delete')), client_updated_at timestamptz not null, payload jsonb,
  created_at timestamptz not null default timezone('utc', now()), updated_at timestamptz not null default timezone('utc', now()), unique(user_id, entity_type, entity_id, client_updated_at)
);

create index activities_user_updated_idx on public.activities(user_id, updated_at desc);
create index activity_logs_user_created_idx on public.activity_logs(user_id, created_at desc);
create index activity_logs_activity_created_idx on public.activity_logs(activity_id, created_at desc);
create index daily_reviews_user_date_idx on public.daily_reviews(user_id, review_date desc);
create index transactions_user_occurred_idx on public.transactions(user_id, occurred_at desc);
create index sync_events_user_created_idx on public.sync_events(user_id, created_at desc);

do $$ declare t text; begin
  foreach t in array array['profiles','activities','activity_plans','activity_logs','daily_reviews','finance_accounts','transaction_categories','transactions','sync_events'] loop
    execute format('alter table public.%I enable row level security', t);
    execute format('create policy %I on public.%I for all using (auth.uid() = user_id) with check (auth.uid() = user_id)', t || '_own_rows', t);
    execute format('create trigger %I before update on public.%I for each row execute function public.set_updated_at()', t || '_set_updated_at', t);
  end loop;
end $$;

