# Instance ID (WP_INS_ID)

## Scope

This document covers assigning and troubleshooting instance ids when deploying
multiple instances:

- one set of WPL/OML rules running in several `wparse` instances at once
- several instances writing to the same downstream table, where `wp_event_id`
  must be globally unique
- numbering discipline for scaling, migration and container rebuilds

Single-instance deployments should still read the deployment checklist below:
the default value is not safe once a second instance appears.

## Why an instance id is required

`wp_event_id` is a 64-bit integer with a fixed layout:

```text
 63            32 31        22 21                    0
+----------------+-----------+-----------------------+
| secs (32 bit)  | inst (10) |  per-second counter   |
+----------------+-----------+-----------------------+
|                |           |        (22 bit)       |
+----------------+-----------+-----------------------+
```

- **secs**: Unix timestamp, valid until 2106-02-07; it occupies the high bits,
  so numeric comparison is time order
- **instance**: `WP_INS_ID - 100`, 10 bits
- **per-second counter**: at most 4,194,304 records per second per instance;
  past that the logical second advances

**Uniqueness rests entirely on the instance id.** Two instances with different
`WP_INS_ID` values write to disjoint id ranges and can never collide. If they
share one, the duplicates are *permanent* rather than occasional — which is why
the deployment side must guarantee it.

## Range

| Item | Value |
| --- | --- |
| Valid range | `100 ..= 1123` |
| Max instances | 1024 |
| Default | `100` (when unset) |

**The number is baked into historical data and cannot be renumbered.** Changing
`WP_INS_ID` on a running instance does not create duplicates (the new lane is a
fresh id range), but the instance's history then spans two lanes and
per-instance aggregation splits.

Pre-allocate blocks so ids stay human-readable:

```text
100 - 199   production
200 - 299   staging / canary
300 - 399   test
400 - 499   development / local
```

## Configuration

Resolution order is identical to `WP_GLOBAL_TAGS`, first match wins:

1. process environment (`docker -e` / systemd `Environment=` / K8s `env`)
2. `<work_root>/.env`
3. `<work_root>/.warp_parse/.env`

The process environment wins and `.env` never overrides an existing variable,
so the orchestrator always has the final say.

### systemd

```ini
[Service]
Environment=WP_INS_ID=101
WorkingDirectory=/srv/wp/proj-a
ExecStart=/usr/local/bin/wparse daemon
```

### Docker Compose

Give each service instance its own number:

```yaml
services:
  wparse-a:
    image: wparse:0.36.2-alpha
    environment:
      WP_INS_ID: "101"
    volumes: [ ./proj-a:/work ]
  wparse-b:
    image: wparse:0.36.2-alpha
    environment:
      WP_INS_ID: "102"
    volumes: [ ./proj-b:/work ]
```

### Kubernetes

The pod name of a StatefulSet is `<statefulset>-<ordinal>`, and the controller
guarantees the ordinal is unique. Map it to the instance id with an init
container:

```yaml
initContainers:
  - name: assign-instance-id
    image: busybox:1.36
    command:
      - sh
      - -c
      - |
        ordinal=${HOSTNAME##*-}
        echo "WP_INS_ID=$((100 + ordinal))" > /work/.env
    volumeMounts:
      - { name: work, mountPath: /work }
```

> `HOSTNAME` inside a pod is `metadata.name`; no extra injection is needed.

A Deployment has no ordinal, so ids must be assigned explicitly (one ConfigMap
per replica, or an external registry).

**Do not hash `metadata.uid` or the pod name.** Squeezing a high-entropy name
into 10 bits creates *permanent* collisions: 1000 instances in 1024 slots give an
expected `1000² / (2 × 1024) ≈ 488` colliding pairs — far worse than the random
id scheme it replaces, and each collision is permanent.

If the instance count approaches 1024, plan the expansion early: this field is
10 bits wide and there is no room to "add one more lane".

### Per-instance project directory

```bash
# /srv/wp/proj-a/.env
WP_INS_ID=101
```

```bash
# /srv/wp/proj-a/.warp_parse/.env (equivalent fallback location)
WP_INS_ID=101
```

## Startup validation

| Case | Behaviour |
| --- | --- |
| unset | falls back to `100` and logs a WARN |
| valid | logs `instance id: 101` |
| non-numeric / out of range (<100 or >1123) | **startup fails**, message names the range |
| changed after the first id was issued | **startup fails** |

When unset:

```text
WARN WP_INS_ID not set; using default instance id 100 (multiple instance will collide)
```

When invalid:

```text
ERROR  wparse error
配置错误
detail: invalid WP_INS_ID: WP_INS_ID 必须在 [100, 1123] 之间，当前为 99
```

**Never rely on the default for a multi-instance deployment**: every instance
that forgets to set it gets `100`, and their ids collide 100%.

## Deployment checklist

1. every instance has a distinct `WP_INS_ID`
2. startup logs show `instance id: NNN`, with no `WP_INS_ID not set` warning
3. spot-check one record and confirm the instance segment:

   ```bash
   # id is the decimal value of wp_event_id
   # instance = (id / 2^22) % 2^10 + 100
   echo "($id / 4194304) % 1024 + 100" | bc
   ```

4. after changing an id, make sure the old `.env` or old orchestrator config was
   not reused by the new instance

## Cases that can still duplicate

These cannot be solved inside the process and must be handled at the deployment
or downstream layer:

| Case | Note |
| --- | --- |
| two instances with the same `WP_INS_ID` | permanent duplication; only numbering discipline plus monitoring catches it |
| an image or volume snapshot copied to a second machine | two identical `.env` files yield the same id |
| a child forked without `exec` | shares the parent's counter state (`exec` rebuilds it and is unaffected) |
| downstream column too narrow | 32-bit integer, `double` (loses precision above 2^53) or a short `VARCHAR` makes ids *look* duplicated |

Handled by the engine, no operator action needed:

- **crash and restart inside the same second**: the first id is issued in the
  second after the first call, so the previous run's ids are never replayed
- **wall clock rollback**: the last logical second is reused and the counter
  keeps incrementing, so ids never go backwards
- **single-instance overload** (> 4,194,304 records/s): the logical second
  advances one step instead of wrapping

## Relay chains (multi-stage)

When two instances relay data (A parses, then B parses A's output), decide the id
semantics first:

- **B's rule does not extract `wp_event_id`** (the common case): B issues a new id
  from its own lane. Each stage is unique on its own, but the same event ends up
  with two different ids
- **B's rule extracts `wp_event_id`**: this instance does **not** overwrite the
  upstream value and syncs its internal handle to it, so a record keeps a single
  id and both stages correlate

The second form is “relay without re-identifying”:

```wpl
# B's rule: take A's wp_event_id back as an ordinary field
rule relay {
    (chars:wp_event_id, chars:msg)
}
```

If the upstream value is not a positive integer (empty, `0`, prefixed, …) the field
is kept unchanged, a fresh internal id is issued, and a single WARN is logged.

### Recommendations

| Need | Approach |
| --- | --- |
| cross-stage dedup / correlation | prefer a **business key** (`serialno`, `request_id`, …); do not depend on a generated id |
| per-record lineage | let B extract `wp_event_id` (example above) so the id is stable; or keep A's value in a separate `upstream_event_id` field |
| only “do the two stages add up?” | no extra field needed — compare counts |

Two limits to remember:

- `wp_meta_disable` **cannot** disable `wp_event_id` (only `wp_oml_name` / `wp_event_md5`)
- `gen_msg_id` is hardcoded on in the production path and cannot be turned off in config

So “keep only the upstream id” requires a rule that extracts it, not a switch.

### `wp_oml_name` is different: it has a switch

`wp_event_id` / `wp_src_key` / `wp_src_ip` / `wp_event_md5` keep the upstream value,
but `wp_oml_name` (which OML model produced the record) defaults to **this** stage's
value:

```bash
# default false: tcp/syslog/kafka/file sinks replace any same-named field with this instance's OML name
# true: keep the upstream (original stage) OML name instead of appending
UPSTREAM_OML_NAME_KEEP=true
```

With `false` the existing field is removed before appending, so **two `wp_oml_name`
fields never appear**; with `true` a record that already carries it is passed through.

## Downstream column types

`wp_event_id` is a `u64` throughout and is emitted as a **string** — no signed
integer is involved on the wparse side. What remains is how the receiver creates
the column:

| Column type | Usable until |
| --- | --- |
| `VARCHAR` / `String` | 2106-02-07 |
| `BIGINT UNSIGNED` | 2106-02-07 |
| signed `BIGINT` / `Int64` | 2038-01-19 (id ≈ `secs × 2^32`; crosses `i64::MAX` when secs hits `2^31`) |
| 32-bit integer | not usable |

If the column is a `VARCHAR` and you need to sort by id: lexicographic order only
matches numeric order while the digit count is constant. Ids are 19 digits today
and become 20 around 2044, so sorting breaks across that boundary. Use
`CAST(... AS BIGINT UNSIGNED)` or pad the column to a fixed width (e.g.
`CHAR(20)`).

## Verification case

`wp-examples` ships `core/instance_id`, which covers all of the above:

```bash
cd wp-examples/core/instance_id
./run.sh
```

It checks the default value, the process environment, `.env`, process-env
precedence over `.env`, `WP_GLOBAL_TAGS` sharing the same resolution, and that
four invalid values fail at startup.
