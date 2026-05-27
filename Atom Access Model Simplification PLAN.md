# Atom Access Model Simplification Plan

## Summary

Current Atom docs partially define the right ideas, but not in a clean product model. We will update Atom’s product direction to this simpler model:

```text
Tenant = top boundary
Object Group = object boundary/container
Principal Group = who receives roles
Capability = action
Capability Applicability = valid action/object pairs
Role = actions + where they apply
Assignment = who gets the role
```

This removes the confusing current model where scope exists in roles and policies, capabilities have `resource_kind`, groups do multiple jobs, and composite roles hide access.

## Product Model

### Tenant

Tenant is the top-level boundary.

In MG language:

```text
Tenant = Domain
```

Example:

```text
Tenant: Factory-1
```

### Object Group

Object Group is a container/boundary inside a tenant.

It can contain protected objects:

```text
clients/devices
channels
rules
reports
alarms
child object groups
```

Object Group supports nesting:

```text
Factory-1
  Plant-A
    Line-1
      client1
      channel1
    Line-2
      client2
      channel2
```

Object Group is used only for “where access applies”.

### Principal Group

Principal Group is a collection of identities.

It can contain:

```text
users
services
applications
workloads
devices if needed
```

Principal Group is used only for “who receives access”.

Example:

```text
Principal Group: Operators
Members: user1, user2

Principal Group: MG Services
Members: mg-service, reports-service
```

### Capability

Capability is a global action.

Examples:

```text
read
write
delete
publish
subscribe
execute
role.manage
policy.manage
credential.manage
audit.read
tenant.manage
```

Do not create capability names like:

```text
channel_publish
client_read
report_execute
```

### Capability Applicability

Capability Applicability says which object types support which actions.

It does not grant access.

Examples:

```text
publish -> channel
subscribe -> channel
execute -> rule/report
read/write/delete -> clients/channels/groups/rules/reports/alarms
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

Permissions:
- Applies to: clients in Object Group Plant-A
  Actions: read, write

- Applies to: channels in Object Group Plant-A
  Actions: publish, subscribe
```

### Assignment

Assignment gives a role to a subject.

Subject can be:

```text
single entity
principal group
```

Example:

```text
Assign Plant-A Operator to user1
Assign Plant-A Operator to Principal Group Operators
```

Assignment has no scope. The role already defines where access applies.

## Required Documentation Changes

Update `product-docs/PRD.md` and related docs to replace the old language.

### Replace

```text
Group = collection of entities used for shared access
Role = bundle of capabilities, optionally scoped
Policy = grants role/capability over scope
Capability may apply to resource_kind
```

### With

```text
Object Group = object boundary/container
Principal Group = identity collection
Role = permission blocks
Assignment = role to entity/principal group
Capability = global action
Capability Applicability = action/object validation
```

### Update Examples

Old style:

```text
Policy grants publish capability to group on channel
```

New style:

```text
Role: Channel Publisher
Permission: applies to channel c1, action publish
Assignment: give role to Principal Group Field Devices
```

Old style:

```text
Group has policy
```

New style:

```text
Principal Group receives role
Object Group defines where role applies
```

## Backend Direction

### Final Product Rules

Use these rules as the source of truth for the simplified model:

```text
Normal role always belongs to one tenant.
Platform role is only for system/admin/service use.

Role contains permission blocks.
Assignment only connects subject to role.

Object Group is where.
Principal Group is who.

Assignment has no scope.
Capability is only an action.
Capability Applicability validates action/object type.

Listing uses read-access SQL filtering.
Direct capability grants are internal policy records.
```

### Role Ownership

Normal roles are tenant-owned.

```text
role.tenant_id = tenant.id
```

Tenant users should create only tenant roles.

Tenant-free or platform roles are reserved for:

```text
system admin
Atom internal services
MG service entities
cross-tenant service automation
```

Normal UI must not allow regular users to create tenant-free roles.

### Assignment Rules

Assignment is the human-friendly product name for policy.

An assignment connects:

```text
subject -> role
```

Subject can be:

```text
single entity
principal group
```

Assignment must not have its own scope. The role already defines where access applies.

Validation rules:

```text
role tenant must match subject tenant
principal group tenant must match role tenant
cross-tenant assignment is allowed only for platform/system roles
```

### Deny And Conditions

Deny still overrides allow.

However, deny and conditions should be treated as advanced features.

Normal UI should focus on:

```text
allow role assignment
```

Advanced/internal API can keep:

```text
deny assignments
conditional assignments
```

This keeps the normal product easy to understand while preserving the security model for advanced use cases.

### Listing Rule

Atom should not use a separate `list` capability for normal object listing.

Listing means:

```text
return objects the caller can read
```

This must be implemented through SQL authorization filtering, not by loading every object and checking access one by one.

Examples:

```text
List clients in tenant -> return clients where user has read access
List channels in Object Group -> return channels where user has read access
```

### Group Rules

Object Group supports nesting.

```text
Plant-A
  Line-1
  Line-2
```

Principal Group is flat in V1.

One object belongs to one Object Group in V1.

This keeps boundary evaluation simple and avoids confusing access inheritance.

### Direct Capability Grants

Direct capability grants are not part of normal user-facing access management.

They are trusted internal policy records for machine-created runtime links, for example:

```text
client1 can publish to channel1
client1 can subscribe to channel2
```

Do not add a separate `direct_grants` table. Reuse Atom's internal policy model so this remains generic and not Magistrala-specific.

Conceptual policy shape:

```text
subject = entity or Principal Group
action = capability
object = protected entity/resource/Object Group/tenant
effect = allow or deny
conditions = optional advanced rules
```

Guardrails:

```text
not exposed in normal UI
created only by trusted service/system flows
audited
not used as the main role model
```

Normal UI should still show Roles and Assignments. Policy records are advanced/security internals.

### Schema Direction

Future clean schema should have:

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

Remove from normal product model:

```text
roles.scope_kind
roles.scope_ref
policy scope for role assignments
capabilities.resource_kind
role_capabilities as direct role model
role_composites
```

Advanced/security backing may still use internal policy records for:

```text
role assignment policies
direct capability policies
deny/conditional policies
```

Do not create a separate `direct_grants` table. Direct capability grants are internal policy records for trusted machine-created links, such as strict client-channel publish/subscribe connections.

Current `groups` should not continue as one overloaded concept. It should be split clearly into:

```text
object_groups
principal_groups
```

Do not reuse one table/API called `groups` for both meanings in the public product model.

### Minimum Public API Direction

GraphQL should expose product-level operations around the simplified model:

```text
createRole with permission blocks
updateRole permission blocks
assignRoleToEntity
assignRoleToPrincipalGroup
removeRoleAssignment

createObjectGroup
setObjectGroupParent
addEntityToObjectGroup
addResourceToObjectGroup
removeObjectFromObjectGroup

createPrincipalGroup
addPrincipalGroupMember
removePrincipalGroupMember
```

Do not expose `scope_kind`, `scope_ref`, or policy scope in normal APIs used by MG UI.

## Authorization Flow

When checking access:

```text
Can user1 publish to channel1?
```

Atom evaluates:

1. `publish` is valid for `channel`.
2. user1 has direct role assignment or belongs to Principal Group with role assignment.
3. assigned role has permission block applying to channel1.
4. conditions match.
5. deny overrides allow.
6. otherwise default deny.

Example:

```text
user1 is in Principal Group Operators
Operators has Plant-A Operator role
Plant-A Operator allows publish on channels in Object Group Plant-A
channel1 is inside Plant-A
=> allowed
```

## UI Direction

Normal UI should show only:

```text
Tenants/Domains
Entities
Object Groups
Principal Groups
Roles
Assignments
Capabilities
```

Do not show normal users:

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

## Test Scenarios

- Create Object Group tree:
  - Plant-A
  - Line-1 under Plant-A
- Put client/channel into Object Group.
- Create Principal Group:
  - Operators
- Add users/services to Principal Group.
- Create role with permission blocks for Object Group.
- Assign role to Principal Group.
- Verify members receive access.
- Verify non-members do not receive access.
- Verify Object Group containment alone grants no access.
- Verify invalid capability/object pair is rejected:
  - publish on client
  - execute on channel
- Verify service entity can receive roles directly or through Principal Group.
- Verify service can access multiple tenants through multiple assignments or platform-level role.

## Assumptions

- This is a product-model simplification before production release.
- Breaking compatibility is accepted.
- Existing implementation can be refactored because Atom has no released DB contract.
- Assignment is the human-friendly name for policy.
- Scope is removed from normal product language and replaced by `Applies to`.
- Object Group and Principal Group are separate concepts.
