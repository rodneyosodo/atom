# Atom Access Model

## Status: Draft
## Date: 2026-05-21

This document defines the simplified Atom access model. It is the product direction for new UI, GraphQL, database, and Magistrala integration work.

---

## One-line model

```text
Assign a Role to an Entity or Principal Group.
The Role says what actions apply to which objects inside a Tenant or Object Group.
```

---

## Concepts

### Tenant

Tenant is the top boundary.

In Magistrala:

```text
Tenant = Domain
```

Tenant owns normal application objects such as clients, channels, rules, reports, alarms, Object Groups, Principal Groups, and tenant roles.

### Entity

Entity is an identity.

Kinds:

```text
human
device
service
workload
application
```

An entity can be a subject that receives access. Some entities, such as devices, are also protected objects that other entities can manage.

Human users are global by default. Their tenant participation is represented through tenant membership and role assignments, not by duplicating the same human in each tenant.

Services are entities too. A service can receive roles directly or through a Principal Group, including platform roles for cross-tenant automation.

### Resource

Resource is an application object protected by Atom authorization.

Examples:

```text
channel
rule
report
alarm
custom application object
```

Clients/devices are not resources. They are entities with `kind = device` or `kind = service`.

### Object Group

Object Group is a boundary/container for protected objects inside a tenant.

It answers:

```text
where does this role apply?
```

It can contain:

```text
clients/devices
channels
rules
reports
alarms
child Object Groups
```

Object Group supports nesting:

```text
Tenant Factory-1
  Object Group Plant-A
    Object Group Line-1
      client1
      channel1
    Object Group Line-2
      client2
      channel2
```

Rules:

- Object Group is not a subject that receives roles.
- Object Group containment alone grants no access.
- One object belongs to one Object Group in V1.
- Nested Object Groups are used only for boundary matching.

### Principal Group

Principal Group is a collection of identities.

It answers:

```text
who receives this role?
```

It can contain:

```text
users
services
applications
workloads
devices if needed
```

Examples:

```text
Principal Group Operators = user1, user2
Principal Group MG Services = mg-service, reports-service
```

Rules:

- Principal Group is a subject that can receive role assignments.
- Principal Group is not an object boundary.
- Principal Groups are flat in V1.

### Capability

Capability is a global action name.

Examples:

```text
read
write
delete
publish
subscribe
execute
manage
role.manage
policy.manage
credential.manage
audit.read
tenant.manage
```

Do not create object-specific capability names such as:

```text
client_read
channel_publish
report_execute
```

### Capability Applicability

Capability Applicability says which object types support which actions.

It validates the role model. It does not grant access.

Examples:

```text
publish -> channel
subscribe -> channel
execute -> rule/report
read/write/delete -> clients/channels/groups/rules/reports/alarms
credential.manage -> entity credentials
tenant.manage -> tenant lifecycle
```

If a role tries to add an invalid pair, Atom rejects it.

Examples:

```text
publish on client -> invalid
execute on channel -> invalid
```

### Role

Role contains permission blocks.

Each permission block says:

```text
where it applies + actions
```

Example:

```text
Role: Plant-A Operator

Permission block 1:
  Applies to: clients in Object Group Plant-A
  Actions: read, write

Permission block 2:
  Applies to: channels in Object Group Plant-A
  Actions: read, publish, subscribe
```

Normal roles belong to one tenant:

```text
role.tenant_id = tenant.id
```

Platform roles are tenant-free and reserved for system, admin, and service automation.

### Assignment

Assignment gives a role to a subject.

Subject can be:

```text
single entity
principal group
```

Examples:

```text
Assign Plant-A Operator to user1
Assign Plant-A Operator to Principal Group Operators
Assign MG Cross Tenant Service to mg-service
```

Assignment has no scope. The role already defines where access applies.

Rules:

- Role tenant must match subject tenant context.
- Principal Group tenant must match role tenant.
- Cross-tenant assignment is allowed only for platform/system roles.
- Normal UI should show "assignment", not "policy binding".

---

## Authorization Flow

Question:

```text
Can user1 publish to channel1?
```

Atom evaluates:

1. `publish` is valid for `channel`.
2. `user1` is active.
3. `user1` has a direct role assignment or belongs to a Principal Group with a role assignment.
4. The assigned role has a permission block that applies to `channel1`.
5. Optional advanced conditions match.
6. Any matching deny overrides allow.
7. If no allow matches, deny.

Example:

```text
user1 is in Principal Group Operators
Operators has Plant-A Operator role
Plant-A Operator allows publish on channels in Object Group Plant-A
channel1 is inside Plant-A
=> allowed
```

---

## Listing Rule

Atom should not require a separate `list` capability for normal object listing.

Listing means:

```text
return objects the caller can read
```

This must be implemented with SQL authorization filtering. Atom must not load every object and call the PDP one by one.

Examples:

```text
List clients in tenant -> return clients where caller has read access
List channels in Object Group -> return channels where caller has read access
```

---

## Advanced Features

Deny and conditions remain part of the security model, but normal UI should focus on allow assignments.

### Internal Policy Records

Under the hood, Atom may use internal policy records for advanced/security behavior. This keeps the normal product model simple while allowing precise runtime links and security controls.

Internal policy record types:

```text
role assignment policy = gives a role to an entity or Principal Group
direct capability policy = trusted subject/action/object grant
deny/conditional policy = advanced security rule
```

Direct capability grants are trusted internal policy records. They may be used for machine-created runtime links, such as strict client-channel publish/subscribe connections.

Conceptual direct capability policy shape:

```text
subject = entity or Principal Group
action = capability
object = protected entity/resource/Object Group/tenant
effect = allow or deny
conditions = optional advanced rules
```

Example for a Magistrala client-channel link:

```text
subject = client entity
action = publish or subscribe
object = channel resource
effect = allow
```

Guardrails:

- Not exposed in normal UI.
- Created only by trusted service/system flows.
- Audited.
- Not used as the main role model.
- No separate `direct_grants` table; direct capability grants reuse the internal policy model.

---

## Clean Schema Direction

Future clean schema should model these concepts directly:

```text
tenants
entities
resources
object_groups
object_group_hierarchy
object_group_entities
object_group_resources
principal_groups
principal_group_members
capabilities
capability_applicability
roles
role_permission_blocks
role_permission_actions
role_assignments
```

The normal product model should remove:

```text
roles.scope_kind
roles.scope_ref
policy scope for role assignments
capabilities.resource_kind
role_capabilities as the direct role model
role_composites
one overloaded groups table/API
```

Advanced/security backing may still use internal policy records. Normal UI must hide policy internals and expose Roles, Assignments, Principal Groups, and Object Groups instead.

---

## UI Language

Normal UI should show:

```text
Tenants / Domains
Entities
Resources
Object Groups
Principal Groups
Roles
Assignments
Capabilities
```

Normal UI should not show:

```text
scope_kind
scope_ref
policy scope
resource_kind on capability
simple role
composite role
direct capability policy
```

Role form should look like:

```text
Role name: Plant-A Operator

Permissions:
[Applies to: Clients in Plant-A] [Actions: read, write]
[Applies to: Channels in Plant-A] [Actions: publish, subscribe]

Assignments:
[user1]
[Principal Group Operators]
```
