// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

import {IAccessController} from "../interfaces/IAccessController.sol";

contract AccessController is IAccessController {
    address public admin;
    bool private _paused;

    mapping(address => bool) private _workflowPublishers;
    mapping(address => bool) private _policyAdmins;
    mapping(address => bool) private _verifierAdmins;
    mapping(address => bool) private _pausers;

    constructor(address admin_) {
        if (admin_ == address(0)) revert InvalidAddress();
        admin = admin_;
        _pausers[admin_] = true;
        _policyAdmins[admin_] = true;
        _verifierAdmins[admin_] = true;
    }

    modifier onlyAdmin() {
        _onlyAdmin();
        _;
    }

    function _onlyAdmin() internal view {
        if (msg.sender != admin) revert Unauthorized();
    }

    function setWorkflowPublisher(address account, bool allowed) external onlyAdmin {
        if (account == address(0)) revert InvalidAddress();
        if (_workflowPublishers[account] == allowed) revert AlreadySet();
        _workflowPublishers[account] = allowed;
        emit WorkflowPublisherSet(account, allowed);
    }

    function setPolicyAdmin(address account, bool allowed) external onlyAdmin {
        if (account == address(0)) revert InvalidAddress();
        if (_policyAdmins[account] == allowed) revert AlreadySet();
        _policyAdmins[account] = allowed;
        emit PolicyAdminSet(account, allowed);
    }

    function setVerifierAdmin(address account, bool allowed) external onlyAdmin {
        if (account == address(0)) revert InvalidAddress();
        if (_verifierAdmins[account] == allowed) revert AlreadySet();
        _verifierAdmins[account] = allowed;
        emit VerifierAdminSet(account, allowed);
    }

    function setPauser(address account, bool allowed) external onlyAdmin {
        if (account == address(0)) revert InvalidAddress();
        if (_pausers[account] == allowed) revert AlreadySet();
        _pausers[account] = allowed;
        emit PauserSet(account, allowed);
    }

    function isWorkflowPublisher(address account) external view returns (bool) {
        return _workflowPublishers[account];
    }

    function isPolicyAdmin(address account) external view returns (bool) {
        return _policyAdmins[account];
    }

    function isVerifierAdmin(address account) external view returns (bool) {
        return _verifierAdmins[account];
    }

    function isPauser(address account) external view returns (bool) {
        return _pausers[account];
    }

    function setPaused(bool paused_) external {
        if (!_pausers[msg.sender] && msg.sender != admin) revert Unauthorized();
        if (_paused == paused_) revert AlreadySet();
        _paused = paused_;
        emit PauseStateSet(paused_);
    }

    function paused() external view returns (bool) {
        return _paused;
    }
}
