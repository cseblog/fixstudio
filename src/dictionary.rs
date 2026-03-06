// ---------------------------------------------------------------------------
// FIX tag & value dictionaries
// ---------------------------------------------------------------------------

/// Human-readable message type label from tag 35 value.
pub fn msg_type_label(code: &str) -> &'static str {
    match code {
        "0" => "Heartbeat",
        "1" => "TestRequest",
        "2" => "ResendRequest",
        "3" => "Reject",
        "4" => "SequenceReset",
        "5" => "Logout",
        "6" => "IOI",
        "7" => "Advertisement",
        "8" => "ExecutionReport",
        "9" => "OrderCancelReject",
        "A" => "Logon",
        "B" => "News",
        "C" => "Email",
        "D" => "NewOrderSingle",
        "E" => "NewOrderList",
        "F" => "OrderCancelRequest",
        "G" => "OrderCancelReplaceRequest",
        "H" => "OrderStatusRequest",
        "J" => "AllocationInstruction",
        "K" => "ListCancelRequest",
        "L" => "ListExecute",
        "M" => "ListStatusRequest",
        "N" => "ListStatus",
        "P" => "AllocationInstructionAck",
        "Q" => "DontKnowTrade",
        "R" => "QuoteRequest",
        "S" => "Quote",
        "T" => "SettlementInstructions",
        "V" => "MarketDataRequest",
        "W" => "MarketDataSnapshotFullRefresh",
        "X" => "MarketDataIncrementalRefresh",
        "Y" => "MarketDataRequestReject",
        "Z" => "QuoteCancel",
        "AA" => "QuoteAcknowledgement",
        "AB" => "SecurityDefinitionRequest",
        "AE" => "TradeCaptureReport",
        "AG" => "TradeCaptureReportAck",
        "AI" => "QuoteStatusReport",
        "AK" => "Confirmation",
        "AL" => "PositionMaintenanceRequest",
        "AM" => "PositionMaintenanceReport",
        "AN" => "RequestForPositions",
        "AO" => "RequestForPositionsAck",
        "AP" => "PositionReport",
        "AQ" => "TradeCaptureReportRequestAck",
        "AR" => "TradeCaptureReportAck",
        "AS" => "AllocationReport",
        "AT" => "AllocationReportAck",
        "AU" => "ConfirmationAck",
        "j" => "BusinessMessageReject",
        "BE" => "UserRequest",
        "BF" => "UserResponse",
        "BJ" => "DerivativeSecurityList",
        "BK" => "DerivativeSecurityListRequest",
        _ => "Unknown",
    }
}

/// Human-readable side label from tag 54 value.
pub fn side_label(code: &str) -> &'static str {
    match code {
        "1" => "BUY",
        "2" => "SELL",
        "3" => "BUY MINUS",
        "4" => "SELL PLUS",
        "5" => "SELL SHORT",
        "6" => "SELL SHORT EXEMPT",
        "7" => "UNDISCLOSED",
        "8" => "CROSS",
        "9" => "CROSS SHORT",
        _ => "",
    }
}

/// Tag number -> tag name.
pub fn tag_description(tag: u16) -> &'static str {
    match tag {
        1 => "Account",
        6 => "AvgPx",
        7 => "BeginSeqNo",
        8 => "BeginString",
        9 => "BodyLength",
        10 => "CheckSum",
        11 => "ClOrdID",
        14 => "CumQty",
        15 => "Currency",
        16 => "EndSeqNo",
        17 => "ExecID",
        18 => "ExecInst",
        19 => "ExecRefID",
        20 => "ExecTransType",
        21 => "HandlInst",
        22 => "SecurityIDSource",
        23 => "IOIID",
        25 => "IOIQltyInd",
        27 => "IOIQty",
        28 => "IOITransType",
        29 => "LastCapacity",
        30 => "LastMkt",
        31 => "LastPx",
        32 => "LastQty",
        33 => "NoLinesOfText",
        34 => "MsgSeqNum",
        35 => "MsgType",
        36 => "NewSeqNo",
        37 => "OrderID",
        38 => "OrderQty",
        39 => "OrdStatus",
        40 => "OrdType",
        41 => "OrigClOrdID",
        43 => "PossDupFlag",
        44 => "Price",
        45 => "RefSeqNum",
        47 => "Rule80A",
        48 => "SecurityID",
        49 => "SenderCompID",
        50 => "SenderSubID",
        52 => "SendingTime",
        53 => "Quantity",
        54 => "Side",
        55 => "Symbol",
        56 => "TargetCompID",
        57 => "TargetSubID",
        58 => "Text",
        59 => "TimeInForce",
        60 => "TransactTime",
        61 => "Urgency",
        63 => "SettlType",
        64 => "SettlDate",
        65 => "SymbolSfx",
        75 => "TradeDate",
        76 => "ExecBroker",
        77 => "PositionEffect",
        78 => "NoAllocs",
        79 => "AllocAccount",
        80 => "AllocQty",
        97 => "PossResend",
        98 => "EncryptMethod",
        99 => "StopPx",
        100 => "ExDestination",
        102 => "CxlRejReason",
        103 => "OrdRejReason",
        108 => "HeartBtInt",
        109 => "ClientID",
        110 => "MinQty",
        111 => "MaxFloor",
        112 => "TestReqID",
        114 => "LocateReqd",
        115 => "OnBehalfOfCompID",
        116 => "OnBehalfOfSubID",
        117 => "QuoteID",
        122 => "OrigSendingTime",
        123 => "GapFillFlag",
        126 => "ExpireTime",
        127 => "DKReason",
        128 => "DeliverToCompID",
        129 => "DeliverToSubID",
        131 => "QuoteReqID",
        132 => "BidPx",
        133 => "OfferPx",
        134 => "BidSize",
        135 => "OfferSize",
        141 => "ResetSeqNumFlag",
        142 => "SenderLocationID",
        143 => "TargetLocationID",
        144 => "OnBehalfOfLocationID",
        145 => "DeliverToLocationID",
        150 => "ExecType",
        151 => "LeavesQty",
        152 => "CashOrderQty",
        167 => "SecurityType",
        168 => "EffectiveTime",
        198 => "SecondaryOrderID",
        200 => "MaturityMonthYear",
        201 => "PutOrCall",
        202 => "StrikePrice",
        207 => "SecurityExchange",
        210 => "MaxShow",
        211 => "PegOffsetValue",
        229 => "TradeOriginationDate",
        262 => "MDReqID",
        263 => "SubscriptionRequestType",
        264 => "MarketDepth",
        267 => "NoMDEntryTypes",
        268 => "NoMDEntries",
        269 => "MDEntryType",
        270 => "MDEntryPx",
        271 => "MDEntrySize",
        272 => "MDEntryDate",
        273 => "MDEntryTime",
        274 => "TickDirection",
        275 => "MDMkt",
        276 => "QuoteCondition",
        277 => "TradeCondition",
        278 => "MDEntryID",
        279 => "MDUpdateAction",
        280 => "MDEntryRefID",
        281 => "MDReqRejReason",
        282 => "MDEntryOriginator",
        283 => "LocationID",
        284 => "DeskID",
        286 => "OpenCloseSettlFlag",
        290 => "MDEntryPositionNo",
        336 => "TradingSessionID",
        340 => "TradSesStatus",
        371 => "RefTagID",
        372 => "RefMsgType",
        373 => "SessionRejectReason",
        378 => "ExecRestatementReason",
        382 => "NoMiscFees",
        383 => "MaxMessageSize",
        384 => "NoMsgTypes",
        385 => "MsgDirection",
        409 => "LiquidityIndType",
        410 => "WtAverageLiquidity",
        423 => "PriceType",
        424 => "DayOrderQty",
        425 => "DayCumQty",
        426 => "DayAvgPx",
        427 => "GTBookingInst",
        429 => "ListStatusType",
        430 => "NetGrossInd",
        431 => "ListOrderStatus",
        432 => "ExpireDate",
        433 => "ListExecInstType",
        434 => "CxlRejResponseTo",
        439 => "ClearingFirm",
        440 => "ClearingAccount",
        442 => "MultiLegReportingType",
        447 => "PartyIDSource",
        448 => "PartyID",
        452 => "PartyRole",
        453 => "NoPartyIDs",
        460 => "Product",
        461 => "CFICode",
        526 => "SecondaryClOrdID",
        527 => "SecondaryExecID",
        528 => "OrderCapacity",
        529 => "OrderRestrictions",
        530 => "MassCancelRequestType",
        531 => "MassCancelResponse",
        532 => "MassCancelRejectReason",
        533 => "TotalAffectedOrders",
        571 => "TradeReportID",
        572 => "TradeReportRefID",
        573 => "MatchStatus",
        574 => "MatchType",
        584 => "MassStatusReqID",
        585 => "MassStatusReqType",
        636 => "WorkingIndicator",
        660 => "AcctIDSource",
        693 => "PosReqID",
        702 => "NoPositions",
        703 => "PosType",
        704 => "LongQty",
        705 => "ShortQty",
        706 => "QuantityType",
        707 => "PosAmtType",
        708 => "PosAmt",
        710 => "PosReqID",
        715 => "ClearingBusinessDate",
        721 => "PosMaintRptID",
        722 => "PosMaintStatus",
        724 => "PosReqType",
        727 => "TotalNumPosReports",
        728 => "PosReqResult",
        730 => "SettlPrice",
        731 => "SettlPriceType",
        753 => "NoPosAmt",
        912 => "LastRptRequested",
        _ => "Unknown",
    }
}

/// Resolve a human-readable description for a tag's value.
/// Values sourced from FIX Trading Community specification (fix.dev, fixtrading.org).
pub fn value_description(tag: u16, value: &str) -> &'static str {
    match tag {
        35 => msg_type_label(value),
        39 => ord_status_label(value),
        40 => ord_type_label(value),
        54 => side_label(value),
        59 => time_in_force_label(value),
        150 => exec_type_label(value),
        21 => handl_inst_label(value),
        98 => encrypt_method_label(value),
        22 => security_id_source_label(value),
        20 => exec_trans_type_label(value),
        102 => cxl_rej_reason_label(value),
        103 => ord_rej_reason_label(value),
        373 => session_reject_reason_label(value),
        77 => position_effect_label(value),
        167 => security_type_label(value),
        263 => subscription_request_type_label(value),
        269 => md_entry_type_label(value),
        274 => tick_direction_label(value),
        279 => md_update_action_label(value),
        378 => exec_restatement_reason_label(value),
        434 => cxl_rej_response_to_label(value),
        460 => product_label(value),
        528 => order_capacity_label(value),
        201 => put_or_call_label(value),
        28 => ioi_trans_type_label(value),
        29 => last_capacity_label(value),
        63 => settl_type_label(value),
        385 => msg_direction_label(value),
        _ => "",
    }
}

fn ord_status_label(value: &str) -> &'static str {
    match value {
        "0" => "New",
        "1" => "Partially filled",
        "2" => "Filled",
        "3" => "Done for day",
        "4" => "Canceled",
        "5" => "Replaced",
        "6" => "Pending Cancel",
        "7" => "Stopped",
        "8" => "Rejected",
        "9" => "Suspended",
        "A" => "Pending New",
        "B" => "Calculated",
        "C" => "Expired",
        "D" => "Accepted for bidding",
        "E" => "Pending Replace",
        _ => "",
    }
}

fn ord_type_label(value: &str) -> &'static str {
    match value {
        "1" => "Market",
        "2" => "Limit",
        "3" => "Stop",
        "4" => "Stop Limit",
        "5" => "Market On Close",
        "6" => "With Or Without",
        "7" => "Limit Or Better",
        "8" => "Limit With Or Without",
        "9" => "On Basis",
        "A" => "On Close",
        "B" => "Limit On Close",
        "C" => "Forex Market",
        "D" => "Previously Quoted",
        "E" => "Previously Indicated",
        "F" => "Forex Limit",
        "G" => "Forex Previously Quoted",
        "H" => "Funari",
        "I" => "Market If Touched",
        "J" => "Market With Left Over as Limit",
        "K" => "Previous Fund Valuation Point",
        "L" => "Next Fund Valuation Point",
        "M" => "Pegged",
        "P" => "Pegged",
        _ => "",
    }
}

fn time_in_force_label(value: &str) -> &'static str {
    match value {
        "0" => "DAY",
        "1" => "GTC (Good Till Cancel)",
        "2" => "OPG (At the Opening)",
        "3" => "IOC (Immediate or Cancel)",
        "4" => "FOK (Fill or Kill)",
        "5" => "GTX (Good Till Crossing)",
        "6" => "GTD (Good Till Date)",
        "7" => "At the Close",
        "8" => "Good Through Crossing",
        "9" => "At Crossing",
        _ => "",
    }
}

fn exec_type_label(value: &str) -> &'static str {
    match value {
        "0" => "New",
        "1" => "Partial fill (deprecated)",
        "2" => "Fill (deprecated)",
        "3" => "Done for day",
        "4" => "Canceled",
        "5" => "Replaced",
        "6" => "Pending Cancel",
        "7" => "Stopped",
        "8" => "Rejected",
        "9" => "Suspended",
        "A" => "Pending New",
        "B" => "Calculated",
        "C" => "Expired",
        "D" => "Restated",
        "E" => "Pending Replace",
        "F" => "Trade",
        "G" => "Trade Correct",
        "H" => "Trade Cancel",
        "I" => "Order Status",
        _ => "",
    }
}

fn handl_inst_label(value: &str) -> &'static str {
    match value {
        "1" => "Automated execution, no intervention",
        "2" => "Automated execution, intervention OK",
        "3" => "Manual order",
        _ => "",
    }
}

fn encrypt_method_label(value: &str) -> &'static str {
    match value {
        "0" => "None / Other",
        "1" => "PKCS",
        "2" => "DES",
        "3" => "PKCS/DES",
        "4" => "PGP/DES",
        "5" => "PGP/DES-MD5",
        "6" => "PEM/DES-MD5",
        _ => "",
    }
}

fn security_id_source_label(value: &str) -> &'static str {
    match value {
        "1" => "CUSIP",
        "2" => "SEDOL",
        "3" => "QUIK",
        "4" => "ISIN",
        "5" => "RIC",
        "6" => "ISO Currency Code",
        "7" => "ISO Country Code",
        "8" => "Exchange Symbol",
        "9" => "Consolidated Tape Association",
        _ => "",
    }
}

fn exec_trans_type_label(value: &str) -> &'static str {
    match value {
        "0" => "NEW",
        "1" => "CANCEL",
        "2" => "CORRECT",
        "3" => "STATUS",
        _ => "",
    }
}

fn cxl_rej_reason_label(value: &str) -> &'static str {
    match value {
        "0" => "Too late to cancel",
        "1" => "Unknown order",
        "2" => "Broker / Exchange Option",
        "3" => "Order already in Pending Cancel or Pending Replace",
        "4" => "Unable to process Order Mass Cancel Request",
        "5" => "OrigOrdModTime did not match last TransactTime",
        "6" => "Duplicate ClOrdID received",
        "7" => "Price exceeds current price",
        "8" => "Price exceeds current price band",
        "18" => "Invalid price increment",
        "99" => "Other",
        _ => "",
    }
}

fn ord_rej_reason_label(value: &str) -> &'static str {
    match value {
        "0" => "Broker / Exchange option",
        "1" => "Unknown symbol",
        "2" => "Exchange closed",
        "3" => "Order exceeds limit",
        "4" => "Too late to enter",
        "5" => "Unknown order",
        "6" => "Duplicate Order (e.g. dupe ClOrdID)",
        "7" => "Duplicate of a verbally communicated order",
        "8" => "Stale order",
        "9" => "Trade along required",
        "10" => "Invalid Investor ID",
        "11" => "Unsupported order characteristic",
        "12" => "Surveillance option",
        "13" => "Incorrect quantity",
        "14" => "Incorrect allocated quantity",
        "15" => "Unknown account(s)",
        "16" => "Price exceeds current price band",
        "18" => "Invalid price increment",
        "19" => "Reference price not available",
        "20" => "Notional value exceeds threshold",
        "21" => "Algorithm risk threshold breached",
        "22" => "Short sell not permitted",
        "23" => "Short sell rejected (security pre-borrow)",
        "24" => "Short sell rejected (account pre-borrow)",
        "25" => "Insufficient credit limit",
        "26" => "Exceeded clip size limit",
        "27" => "Exceeded maximum notional order amount",
        "28" => "Exceeded DV01/PV01 limit",
        "29" => "Exceeded CS01 limit",
        "99" => "Other",
        _ => "",
    }
}

fn session_reject_reason_label(value: &str) -> &'static str {
    match value {
        "0" => "Invalid Tag Number",
        "1" => "Required Tag Missing",
        "2" => "Tag not defined for this message type",
        "3" => "Undefined tag",
        "4" => "Tag specified without a value",
        "5" => "Value is incorrect (out of range) for this tag",
        "6" => "Incorrect data format for value",
        "7" => "Decryption problem",
        "8" => "Signature problem",
        "9" => "CompID problem",
        "10" => "SendingTime Accuracy Problem",
        "11" => "Invalid MsgType",
        "12" => "XML Validation Error",
        "13" => "Tag appears more than once",
        "14" => "Tag specified out of required order",
        "15" => "Repeating group fields out of order",
        "16" => "Incorrect NumInGroup count for repeating group",
        "17" => "Non Data value includes field delimiter",
        "18" => "Invalid/Unsupported Application Version",
        "99" => "Other",
        _ => "",
    }
}

fn position_effect_label(value: &str) -> &'static str {
    match value {
        "O" => "Open",
        "C" => "Close",
        "F" => "FIFO",
        "R" => "Rolled",
        _ => "",
    }
}

fn security_type_label(value: &str) -> &'static str {
    match value {
        "ABS" => "Asset-backed securities",
        "AMENDED" => "Amended & Restated",
        "AN" => "Anticipation Notes",
        "BA" => "Bankers Acceptance",
        "BOND" => "Bond",
        "BRADY" => "Brady Bond",
        "BRIDGE" => "Bridge Loan",
        "CD" => "Certificate of Deposit",
        "CMBS" => "CMBS",
        "COFO" => "Certificate Of Obligation",
        "COFP" => "Certificate of Participation",
        "CORP" => "Corporate Bond",
        "CP" => "Commercial Paper",
        "CPP" => "Corporate Private Placement",
        "CS" => "Common Stock",
        "DEFLTED" => "Defaulted",
        "DINP" => "Debtor in Possession",
        "DN" => "Deposit Notes",
        "DUAL" => "Dual Currency",
        "EUCD" => "Euro Certificate of Deposit",
        "EUCORP" => "Euro Corporate Bond",
        "EUSOV" => "Euro Sovereign",
        "EUSUPRA" => "Euro Supranational Coupons",
        "FAC" => "Federal Agency Coupon",
        "FAD" => "Federal Agency Discount Note",
        "FOR" => "Foreign Exchange Contract",
        "FUT" => "Future",
        "GIC" => "Guaranteed Investment Contract",
        "GOVT" => "Government",
        "IET" => "IOETTE",
        "LOFC" => "Letter of Credit",
        "LQN" => "Liquidity Note",
        "MATURED" => "Matured",
        "MF" => "Mutual Fund",
        "MBS" => "Mortgage-backed securities",
        "MIO" => "Mortgage Interest Only",
        "MPO" => "Mortgage Principal Only",
        "MPP" => "Mortgage Private Placement",
        "MPT" => "Miscellaneous Pass-through",
        "MTN" => "Medium Term Notes",
        "MUNIC" => "Municipal",
        "NONE" => "No Security Type",
        "OPT" => "Option",
        "PEF" => "Private Export Funding",
        "PFAND" => "Pfandbriefe",
        "PS" => "Preferred Stock",
        "REPO" => "Repurchase",
        "RETIRED" => "Retired",
        "REV" => "Revenue Bonds",
        "RVLV" => "Revolver Loan",
        "RVLVTRM" => "Revolver/Term Loan",
        "SECLOAN" => "Secured Loan",
        "SLE" => "Sale Leaseback",
        "SLQ" => "Student Loan Marketing Assoc",
        "STN" => "Short Term Loan Note",
        "STRUCT" => "Structured Notes",
        "SUPRA" => "USD Supranational Coupons",
        "SVERN" => "Sovereign",
        "TAN" => "Tax Anticipation Note",
        "TAXA" => "Tax Allocation",
        "TBD" => "To Be Announced",
        "TECP" => "Tax Exempt Commercial Paper",
        "TRAN" => "Transmission",
        "TERM" => "Term Loan",
        "UST" => "US Treasury",
        "USTB" => "US Treasury Bill",
        "WAR" => "Warrant",
        "WITHDRN" => "Withdrawn",
        "WOF" => "Wellesley Off Shore Fund",
        _ => "",
    }
}

fn subscription_request_type_label(value: &str) -> &'static str {
    match value {
        "0" => "Snapshot",
        "1" => "Snapshot + Updates (Subscribe)",
        "2" => "Disable previous Snapshot + Update (Unsubscribe)",
        _ => "",
    }
}

fn md_entry_type_label(value: &str) -> &'static str {
    match value {
        "0" => "Bid",
        "1" => "Offer",
        "2" => "Trade",
        "3" => "Index value",
        "4" => "Opening price",
        "5" => "Closing price",
        "6" => "Settlement price",
        "7" => "Trading session high price",
        "8" => "Trading session low price",
        "9" => "Volume Weighted Average Price",
        "A" => "Imbalance",
        "B" => "Trade volume",
        "C" => "Open interest",
        "D" => "Composite underlying price",
        "H" => "Mid-price",
        "J" => "Empty book",
        "Q" => "Auction clearing price",
        "W" => "Fixing price",
        "t" => "Time Weighted Average Price",
        _ => "",
    }
}

fn tick_direction_label(value: &str) -> &'static str {
    match value {
        "0" => "Plus Tick",
        "1" => "Zero-Plus Tick",
        "2" => "Minus Tick",
        "3" => "Zero-Minus Tick",
        _ => "",
    }
}

fn md_update_action_label(value: &str) -> &'static str {
    match value {
        "0" => "New",
        "1" => "Change",
        "2" => "Delete",
        "3" => "Delete Thru",
        "4" => "Delete From",
        "5" => "Overlay",
        _ => "",
    }
}

fn exec_restatement_reason_label(value: &str) -> &'static str {
    match value {
        "0" => "GT corporate action",
        "1" => "GT renewal / restatement (no corporate action)",
        "2" => "Verbal change",
        "3" => "Repricing of order",
        "6" => "Cancel on Trading Halt",
        "7" => "Cancel on System Failure",
        "9" => "Canceled, not best",
        "12" => "Cancel On Connection Loss",
        "13" => "Cancel On Logout",
        "99" => "Other",
        _ => "",
    }
}

fn cxl_rej_response_to_label(value: &str) -> &'static str {
    match value {
        "1" => "Order Cancel Request (35=F)",
        "2" => "Order Cancel/Replace Request (35=G)",
        _ => "",
    }
}

fn product_label(value: &str) -> &'static str {
    match value {
        "1" => "AGENCY",
        "2" => "COMMODITY",
        "3" => "CORPORATE",
        "4" => "CURRENCY",
        "5" => "EQUITY",
        "6" => "GOVERNMENT",
        "7" => "INDEX",
        "8" => "LOAN",
        "9" => "MONEYMARKET",
        "10" => "MORTGAGE",
        "11" => "MUNICIPAL",
        "12" => "OTHER",
        "13" => "FINANCING",
        _ => "",
    }
}

fn order_capacity_label(value: &str) -> &'static str {
    match value {
        "A" => "Agency",
        "G" => "Proprietary",
        "I" => "Individual",
        "P" => "Principal",
        "R" => "Riskless Principal",
        "W" => "Agent for Other Member",
        "M" => "Mixed Capacity",
        _ => "",
    }
}

fn put_or_call_label(value: &str) -> &'static str {
    match value {
        "0" => "Put",
        "1" => "Call",
        _ => "",
    }
}

fn msg_direction_label(value: &str) -> &'static str {
    match value {
        "S" => "Send",
        "R" => "Receive",
        _ => "",
    }
}

fn ioi_trans_type_label(value: &str) -> &'static str {
    match value {
        "N" => "New",
        "C" => "Cancel",
        "R" => "Replace",
        _ => "",
    }
}

fn last_capacity_label(value: &str) -> &'static str {
    match value {
        "1" => "Agent",
        "2" => "Cross as agent",
        "3" => "Cross as principal",
        "4" => "Principal",
        "5" => "Riskless principal",
        _ => "",
    }
}

fn settl_type_label(value: &str) -> &'static str {
    match value {
        "0" => "Regular / FX Spot (T+1 or T+2)",
        "1" => "Cash (TOD / T+0)",
        "2" => "Next Day (TOM / T+1)",
        "3" => "T+2",
        "4" => "T+3",
        "5" => "T+4",
        "6" => "Future",
        "7" => "When And If Issued",
        "8" => "Sellers Option",
        "9" => "T+5",
        "B" => "Broken date",
        "C" => "FX Spot Next (Spot+1)",
        _ => "",
    }
}

// ---------------------------------------------------------------------------
// Badge / UI helpers used by components
// ---------------------------------------------------------------------------

/// CSS class for the message-type badge in the timeline.
pub fn badge_class(msg_type_raw: &str) -> &'static str {
    match msg_type_raw {
        "A" => "badge-green",  // Logon
        "5" => "badge-red",    // Logout
        "0" => "badge-gray",   // Heartbeat
        "1" => "badge-gray",   // TestRequest
        "D" => "badge-orange", // NewOrderSingle
        "F" => "badge-orange", // OrderCancelRequest
        "G" => "badge-orange", // OrderCancelReplaceRequest
        "8" => "badge-green",  // ExecutionReport
        "9" => "badge-red",    // OrderCancelReject
        "3" => "badge-red",    // Reject
        _ => "badge-blue",
    }
}

/// CSS class for the tag-description badge in the detail panel.
pub fn tag_badge_class(tag: u16) -> &'static str {
    match tag {
        8 | 9 | 10 | 34 => "badge-slate",
        35 => "badge-orange",
        49 | 56 => "badge-blue",
        52 | 60 => "badge-teal",
        55 | 48 | 22 => "badge-purple",
        54 | 38 | 44 | 40 => "badge-green",
        150 | 39 | 37 | 17 => "badge-yellow",
        _ => "badge-slate",
    }
}

/// Returns `true` for tags that are considered "common" header fields.
pub fn is_common_tag(tag: u16) -> bool {
    matches!(tag, 8 | 9 | 10 | 34 | 49 | 56 | 52)
}
