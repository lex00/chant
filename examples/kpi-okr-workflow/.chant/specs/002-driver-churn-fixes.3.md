---
type: implementation
status: completed
prompt: standard
depends_on:
  - 002-driver-churn-fixes
---

# Improve team invite UX

Part of driver spec 002-driver-churn-fixes.

## Context

65% of churned users never invited a teammate. Support tickets cite
"can't figure out how to invite my team" and "settings menu is buried."
Team invite is currently nested under Settings > Organization > Members.

Expected impact: ~1.2pp churn reduction.

## Implementation

Make team invites more discoverable:

1. Add team invite to onboarding wizard (step 2)
2. Add persistent "Invite Team" button to sidebar navigation
3. Show "Invite teammates" prompt on empty project views

## Acceptance Criteria

- [x] Team invite form added to onboarding wizard step 2
- [x] "Invite Team" button added to sidebar (always visible)
- [x] Sidebar button opens team invite modal
- [x] Empty project view shows inline invite prompt
- [x] Invite prompt dismissible (tracked per user)
- [x] All invite flows use same backend API
- [x] Invitation emails sent with project context
- [x] Tests passing for all invite entry points
- [x] Analytics tracking added for invite source
- [x] Deployed to staging

## Implementation Notes

- Sidebar button uses existing InviteModal component
- Empty project prompt: "This project is more fun with teammates!"
- Dismiss tracking stored in user_preferences table
- Analytics event: "invite_initiated" with source param
- Invitation email includes project name and inviter
