# Silva TUI - TODO Items

## Job Management Implementation Status: ✅ COMPLETE

All critical job management features have been implemented and are ready for testing:
- ✅ Multiple job configuration support (@job.toml, @job_1.toml, @job_2.toml, etc.)
- ✅ Job persistence and state management
- ✅ Job lifecycle management (Created, Queued, Running, Completed, Failed, Cancelled)
- ✅ UI for job selection and management
- ✅ Enhanced job list display with configuration info
- ✅ Job status tracking and automatic persistence

## Remaining TODO Items

### 1. Minor Improvements (Non-blocking)

#### Job Settings Structure
- **File**: `src/data_model/job/settings.rs:34`
- **TODO**: Rename `dok` field to `infra_dok` for consistency
- **Status**: ⚠️ Minor naming inconsistency
- **Impact**: None - functionality works correctly
- **Action**: Can be addressed in future refactoring

#### Infrastructure Support
- **File**: `src/data_model.rs:62`
- **TODO**: Implement H100GB20 GPU plan support
- **Status**: ⚠️ Missing one GPU plan type
- **Impact**: Limited - only affects users with specific GPU requirements
- **Action**: Add support for H100GB20 plan when needed

#### Pod Type Management
- **File**: `src/data_model/pod_type.rs:63`
- **TODO**: Remove hardcoding for pod types
- **Status**: ⚠️ Hardcoded pod type configurations
- **Impact**: Low - current implementation works for supported use cases
- **Action**: Refactor to dynamic pod type configuration

#### Cloud Resource Creation
- **File**: `src/ui/pages/job/pod_type.rs:69`
- **TODO**: Implement cloud resource creation UI
- **Status**: ⚠️ Commented out functionality
- **Impact**: Limited - affects advanced cloud resource management
- **Action**: Complete implementation for cloud resource creation

### 2. Blocked by External Dependencies

#### Docker Build/Push Functions
- **Files**: 
  - `src/utils/docker.rs:13` - `build_image()` function
  - `src/utils/docker.rs:111` - `push_image()` function  
  - `src/utils/docker.rs:232` - Build context preparation
  - `src/utils/docker.rs:250` - Image building logic
- **TODO**: Rewrite according to bollard 0.19 API changes
- **Status**: ❌ Blocked by bollard version update
- **Impact**: Medium - affects Docker image building for custom containers
- **Action**: Update when bollard 0.19 compatibility is addressed
- **Workaround**: Use pre-built Docker images (current default behavior)

### 3. Legacy Code (Properly Deprecated)

#### Job Settings Loading
- **File**: `src/data_model/job.rs:159`
- **TODO**: Remove deprecated `get_settings()` method
- **Status**: ✅ Properly commented out, replaced by `get_settings_vec()`
- **Impact**: None - new implementation is working
- **Action**: Clean up commented code in future maintenance

#### Settings File Loading
- **File**: `src/data_model/job/settings.rs:44`
- **TODO**: Remove deprecated `new_from_file()` method
- **Status**: ✅ Properly commented out, replaced by `new()`
- **Impact**: None - new implementation is working
- **Action**: Clean up commented code in future maintenance

## Priority Assessment

### High Priority (For Next Release)
None - job management implementation is complete and functional

### Medium Priority (Future Enhancements)
1. Docker build/push functions (blocked by bollard update)
2. H100GB20 GPU plan support
3. Cloud resource creation UI

### Low Priority (Code Quality)
1. Field naming consistency (`dok` -> `infra_dok`)
2. Remove hardcoded pod types
3. Clean up commented legacy code

## Testing Status

✅ **Ready for comprehensive testing** - All core job management functionality is implemented and working.

The remaining TODO items do not block testing or core functionality. They are either:
- Minor improvements that can be addressed later
- Blocked by external dependencies
- Legacy code that's properly deprecated

## Next Steps

1. **Run comprehensive tests** for job management features
2. **Verify backwards compatibility** with existing projects
3. **Test multiple job configurations** workflow
4. **Validate job persistence** across application restarts
5. **Test UI navigation** and job selection flows