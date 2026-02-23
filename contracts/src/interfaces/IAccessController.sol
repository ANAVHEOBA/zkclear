// SPDX-License-Identifier: MIT
pragma solidity ^0.8.13;

interface IAccessController {
    error Unauthorized();
    error InvalidAddress();
    error AlreadySet();

    event WorkflowPublisherSet(address indexed account, bool allowed);
    event PolicyAdminSet(address indexed account, bool allowed);
    event VerifierAdminSet(address indexed account, bool allowed);
    event PauserSet(address indexed account, bool allowed);
    event PauseStateSet(bool paused);

    function setWorkflowPublisher(address account, bool allowed) external;
    function setPolicyAdmin(address account, bool allowed) external;
    function setVerifierAdmin(address account, bool allowed) external;
    function setPauser(address account, bool allowed) external;

    function isWorkflowPublisher(address account) external view returns (bool);
    function isPolicyAdmin(address account) external view returns (bool);
    function isVerifierAdmin(address account) external view returns (bool);
    function isPauser(address account) external view returns (bool);

    function setPaused(bool paused) external;
    function paused() external view returns (bool);
}
