# Security Policy

Thank you for helping keep pg_jsonschema and its users safe.

## Reporting a vulnerability

Please report suspected vulnerabilities through our HackerOne program:
<https://hackerone.com/supabase>. Please do not open public GitHub issues or
pull requests for security problems.

This repository contains pg_jsonschema, a Postgres extension, written in Rust
with pgrx, that validates `json` and `jsonb` values against JSON Schema. It is a
thin wrapper around the `jsonschema` crate. It is available on the hosted
Supabase platform, and anyone can install it in their own Postgres. We accept
reports for both, and we triage them differently (see "Hosted vs. standalone"
below).

## What we consider a vulnerability

In general, we want to hear about anything that lets a database role use
pg_jsonschema to act with more privilege than it was granted. The problem
should be present on the hosted platform or in a default install.

Examples of what this includes (these are illustrations, not a complete list):

- **Memory-safety bugs.** Memory corruption or code execution in the extension
  that a role can trigger with a crafted schema or instance. A crash that only
  ends the caller's own session is not in scope.
- **Unintended access.** The extension reading files or making network
  requests. It is built without the `jsonschema` crate's HTTP and file
  resolvers, so a schema `$ref` to a remote URL or a local file is not fetched.
  Any way to make it do so is a bug.
- **Unsafe privileged functions.** An extension function that runs with elevated
  rights and can be steered by its caller into running SQL as the function's
  owner.

## What we do not consider a vulnerability

Reports that rely on the behavior below will normally be closed as
informative or not applicable.

**By design**

- **Validation is the feature.** Any role that can run the functions can
  supply a schema and an instance and get a result. Schemas are code-like input
  and are expected to come from the person writing the SQL.
- **A schema's validation result is not an authorization decision.** The
  extension reports whether a value matches a schema. Row security and
  grants are what protect data.
- **Behavior of the underlying `jsonschema` crate**, including which drafts it
  supports and how it interprets keywords. Report problems in the crate to its
  maintainers. We handle them here once they are shown to affect pg_jsonschema.

**Prerequisites that make the attack pointless**

- The attacker needs a privileged database role, and the result is something
  that role can already do.
- The attacker needs `anon` or `authenticated` to run DDL or open a direct
  database session. The Data API does not provide that, and those roles cannot
  log in.

**Missing hardening or noise**

- **Expensive validation.** A schema or instance that makes validation slow,
  such as a deeply nested document or a catastrophic regular expression, is
  expected to cost the role that sent it its own time. A report must show an
  effect on other users or tenants.
- **Dependency CVEs** in the `jsonschema` crate or others that are not
  reachable through pg_jsonschema. Show that the vulnerable code runs.

## Out of scope for every Supabase asset

These are excluded program-wide (see also
<https://supabase.com/.well-known/security.txt>). They are repeated here so
you do not need to leave this repository to find them.

- Clickjacking on pages with no sensitive actions.
- Unauthenticated, logout and login CSRF.
- Attacks requiring MITM or physical access to a user's device.
- Attacks requiring social engineering.
- Any activity that could lead to the disruption of our service (DoS).
  This means volumetric attacks, load testing and flooding. A vulnerability in
  our code that lets an unprivileged attacker crash a service or exhaust
  resources for other users or tenants is in scope. Denial of service that only
  affects the attacker's own project, or that needs admin or owner privileges,
  is not.
- Content spoofing and text injection issues without showing an attack vector,
  or without being able to modify HTML/CSS.
- Email spoofing.
- Missing DNSSEC, CAA or CSP headers.
- Lack of Secure or HttpOnly flags on non-sensitive cookies.
- Dead links.
- User enumeration.

## Hosted vs. standalone

On the hosted platform, the boundary is between the roles a customer holds,
including `postgres`, and superuser, which Supabase reserves for itself. On
your own install, the boundary is between the roles you grant access to
pg_jsonschema and superuser. Say which one you tested.

## How to write a report that gets triaged quickly

- **Say what you tested, at the top of the report and in the title.** Use one of
  these labels, for example `[Production] ...` in the title and a "Target" line
  at the start of the description:
  - **Production**: the hosted Supabase platform, against a project you own.
    Include the project ref and the role you connected as. Never test another
    customer's project.
  - **Standalone, default configuration**: pg_jsonschema from this repository, installed in Postgres as the
    README describes, with no changed settings. Include the pg_jsonschema and
    Postgres versions.
  - **Custom configuration**: you changed grants, settings or code. List every
    change and why. We triage these lower.

  Reports that do not say which of the three they are about are slower to
  triage, and are often closed with a request to retest. A finding that works
  only on a custom configuration is not a finding against production.

- **Give a short, runnable PoC.** The exact SQL, run as the role you name, with
  the schema and instance, and output showing the impact.
- **State the attacker's starting position and the boundary you crossed.**
  Which role do they start as, and what do they reach?
- **Include versions.** The pg_jsonschema, `jsonschema` crate and Postgres versions.
- **One issue per report.** Link variants of the same root cause instead of
  filing each variant separately.
- **Do not send unreviewed AI output.** Reports that were not reproduced by a
  human and that ignore the documentation are closed. AI-assisted analysis is
  fine when you have verified the result.
- **Respond to follow-up questions.** Reports that go quiet after we ask for a
  reproduction are closed.

## Severity

We rate severity by realistic impact. Issues that need an existing privilege,
such as direct database credentials, are rated lower even when they are
valid, and we may fix them as defense in depth.

## Testing and disclosure

These are our program-wide rules. They also apply to AI agents and automated
tools acting on your behalf.

**Testing**

- Do not run automated scanners against other customers' projects. Scanners can
  run up costs for our users, can disrupt services, and our own security
  systems cannot tell hostile reconnaissance from research. If you want to run
  an automated scanner, notify security@supabase.com first and run it only on
  your own Supabase project.
- Do not attack projects that belong to other customers.
- Do not take advantage of a vulnerability you find, for example by downloading
  more data than necessary to demonstrate it, or by deleting or modifying other
  people's data.

**Disclosure**

- Do not reveal the problem to others until we have researched and fixed it and
  informed affected customers.
- If you want to share your research publicly (a conference, a blog or any
  other public forum), send us a draft for review and approval at least 30 days
  before the publication date. Do not include data about Supabase customer
  projects, customers' data, or information about Supabase employees,
  contractors or partners.

**What we promise**

- We will respond to your report within 5 business days with our evaluation and
  an expected resolution date.
- If you have followed these instructions, we will not take legal action against
  you in regard to the report.
- We will handle your report with strict confidentiality and will not pass on
  your personal details to third parties without your permission.
- We will keep you informed of our progress.
- In public information about the problem, we will name you as the discoverer
  unless you prefer otherwise.
