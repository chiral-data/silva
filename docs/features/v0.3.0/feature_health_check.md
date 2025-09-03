# Feature: Health Check

## 1. Summary

Check the configuration to ensure silva can run successfully, simply the current configuration which is too complicated.

## 2. User Goal

- end users

## 3. UI Behavior

- **Location:** a new Tab "Health Check"
- **Appearance:**
  - a list of configuration to be checked
  - if it is success, show a green check, othewise show a red cross
- **Interaction:**
  - user can rerun the health check and the results shown in the UI will be updated.

## 4. Tasks

### Configuration items to be checked

- [ ] If SILVA_CHIRAL_USERNAME and SILVA_CHIRAL_API_TOKEN are both set, then the chiral service will be used
- [ ] Otherwise local computer will be used. In this case, "docker" is required to be installed locally.

## 5. Acceptance Criteria

## 6. Scope
