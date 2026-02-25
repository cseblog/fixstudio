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
pub fn tag_description(tag: &str) -> &'static str {
    match tag {
        "1" => "Account",
        "6" => "AvgPx",
        "7" => "BeginSeqNo",
        "8" => "BeginString",
        "9" => "BodyLength",
        "10" => "CheckSum",
        "11" => "ClOrdID",
        "14" => "CumQty",
        "15" => "Currency",
        "16" => "EndSeqNo",
        "17" => "ExecID",
        "18" => "ExecInst",
        "19" => "ExecRefID",
        "20" => "ExecTransType",
        "21" => "HandlInst",
        "22" => "SecurityIDSource",
        "23" => "IOIID",
        "25" => "IOIQltyInd",
        "27" => "IOIQty",
        "28" => "IOITransType",
        "29" => "LastCapacity",
        "30" => "LastMkt",
        "31" => "LastPx",
        "32" => "LastQty",
        "33" => "NoLinesOfText",
        "34" => "MsgSeqNum",
        "35" => "MsgType",
        "36" => "NewSeqNo",
        "37" => "OrderID",
        "38" => "OrderQty",
        "39" => "OrdStatus",
        "40" => "OrdType",
        "41" => "OrigClOrdID",
        "43" => "PossDupFlag",
        "44" => "Price",
        "45" => "RefSeqNum",
        "47" => "Rule80A",
        "48" => "SecurityID",
        "49" => "SenderCompID",
        "50" => "SenderSubID",
        "52" => "SendingTime",
        "53" => "Quantity",
        "54" => "Side",
        "55" => "Symbol",
        "56" => "TargetCompID",
        "57" => "TargetSubID",
        "58" => "Text",
        "59" => "TimeInForce",
        "60" => "TransactTime",
        "61" => "Urgency",
        "63" => "SettlType",
        "64" => "SettlDate",
        "65" => "SymbolSfx",
        "75" => "TradeDate",
        "76" => "ExecBroker",
        "77" => "PositionEffect",
        "78" => "NoAllocs",
        "79" => "AllocAccount",
        "80" => "AllocQty",
        "97" => "PossResend",
        "98" => "EncryptMethod",
        "99" => "StopPx",
        "100" => "ExDestination",
        "102" => "CxlRejReason",
        "103" => "OrdRejReason",
        "108" => "HeartBtInt",
        "109" => "ClientID",
        "110" => "MinQty",
        "111" => "MaxFloor",
        "112" => "TestReqID",
        "114" => "LocateReqd",
        "115" => "OnBehalfOfCompID",
        "116" => "OnBehalfOfSubID",
        "117" => "QuoteID",
        "122" => "OrigSendingTime",
        "123" => "GapFillFlag",
        "126" => "ExpireTime",
        "127" => "DKReason",
        "128" => "DeliverToCompID",
        "129" => "DeliverToSubID",
        "131" => "QuoteReqID",
        "132" => "BidPx",
        "133" => "OfferPx",
        "134" => "BidSize",
        "135" => "OfferSize",
        "141" => "ResetSeqNumFlag",
        "142" => "SenderLocationID",
        "143" => "TargetLocationID",
        "144" => "OnBehalfOfLocationID",
        "145" => "DeliverToLocationID",
        "150" => "ExecType",
        "151" => "LeavesQty",
        "152" => "CashOrderQty",
        "167" => "SecurityType",
        "168" => "EffectiveTime",
        "198" => "SecondaryOrderID",
        "200" => "MaturityMonthYear",
        "201" => "PutOrCall",
        "202" => "StrikePrice",
        "207" => "SecurityExchange",
        "210" => "MaxShow",
        "211" => "PegOffsetValue",
        "229" => "TradeOriginationDate",
        "262" => "MDReqID",
        "263" => "SubscriptionRequestType",
        "264" => "MarketDepth",
        "267" => "NoMDEntryTypes",
        "268" => "NoMDEntries",
        "269" => "MDEntryType",
        "270" => "MDEntryPx",
        "271" => "MDEntrySize",
        "272" => "MDEntryDate",
        "273" => "MDEntryTime",
        "274" => "TickDirection",
        "275" => "MDMkt",
        "276" => "QuoteCondition",
        "277" => "TradeCondition",
        "278" => "MDEntryID",
        "279" => "MDUpdateAction",
        "280" => "MDEntryRefID",
        "281" => "MDReqRejReason",
        "282" => "MDEntryOriginator",
        "283" => "LocationID",
        "284" => "DeskID",
        "286" => "OpenCloseSettlFlag",
        "290" => "MDEntryPositionNo",
        "336" => "TradingSessionID",
        "340" => "TradSesStatus",
        "371" => "RefTagID",
        "372" => "RefMsgType",
        "373" => "SessionRejectReason",
        "378" => "ExecRestatementReason",
        "382" => "NoMiscFees",
        "383" => "MaxMessageSize",
        "384" => "NoMsgTypes",
        "385" => "MsgDirection",
        "409" => "LiquidityIndType",
        "410" => "WtAverageLiquidity",
        "423" => "PriceType",
        "424" => "DayOrderQty",
        "425" => "DayCumQty",
        "426" => "DayAvgPx",
        "427" => "GTBookingInst",
        "429" => "ListStatusType",
        "430" => "NetGrossInd",
        "431" => "ListOrderStatus",
        "432" => "ExpireDate",
        "433" => "ListExecInstType",
        "434" => "CxlRejResponseTo",
        "439" => "ClearingFirm",
        "440" => "ClearingAccount",
        "442" => "MultiLegReportingType",
        "447" => "PartyIDSource",
        "448" => "PartyID",
        "452" => "PartyRole",
        "453" => "NoPartyIDs",
        "460" => "Product",
        "461" => "CFICode",
        "526" => "SecondaryClOrdID",
        "527" => "SecondaryExecID",
        "528" => "OrderCapacity",
        "529" => "OrderRestrictions",
        "530" => "MassCancelRequestType",
        "531" => "MassCancelResponse",
        "532" => "MassCancelRejectReason",
        "533" => "TotalAffectedOrders",
        "571" => "TradeReportID",
        "572" => "TradeReportRefID",
        "573" => "MatchStatus",
        "574" => "MatchType",
        "584" => "MassStatusReqID",
        "585" => "MassStatusReqType",
        "636" => "WorkingIndicator",
        "660" => "AcctIDSource",
        "693" => "PosReqID",
        "702" => "NoPositions",
        "703" => "PosType",
        "704" => "LongQty",
        "705" => "ShortQty",
        "706" => "QuantityType",
        "707" => "PosAmtType",
        "708" => "PosAmt",
        "710" => "PosReqID",
        "715" => "ClearingBusinessDate",
        "721" => "PosMaintRptID",
        "722" => "PosMaintStatus",
        "724" => "PosReqType",
        "727" => "TotalNumPosReports",
        "728" => "PosReqResult",
        "730" => "SettlPrice",
        "731" => "SettlPriceType",
        "753" => "NoPosAmt",
        "912" => "LastRptRequested",
        _ => "Unknown",
    }
}

/// Resolve a human-readable description for a tag's value.
/// Values sourced from FIX Trading Community specification (fix.dev, fixtrading.org).
pub fn value_description(tag: &str, value: &str) -> String {
    match tag {
        "35" => msg_type_label(value).to_string(),
        "39" => ord_status_label(value),
        "40" => ord_type_label(value),
        "54" => side_label(value).to_string(),
        "59" => time_in_force_label(value),
        "150" => exec_type_label(value),
        "21" => handl_inst_label(value),
        "98" => encrypt_method_label(value),
        "22" => security_id_source_label(value),
        "20" => exec_trans_type_label(value),
        "102" => cxl_rej_reason_label(value),
        "103" => ord_rej_reason_label(value),
        "373" => session_reject_reason_label(value),
        "77" => position_effect_label(value),
        "167" => security_type_label(value),
        "263" => subscription_request_type_label(value),
        "269" => md_entry_type_label(value),
        "274" => tick_direction_label(value),
        "279" => md_update_action_label(value),
        "378" => exec_restatement_reason_label(value),
        "434" => cxl_rej_response_to_label(value),
        "460" => product_label(value),
        "528" => order_capacity_label(value),
        "201" => put_or_call_label(value),
        "28" => ioi_trans_type_label(value),
        "29" => last_capacity_label(value),
        "63" => settl_type_label(value),
        "385" => msg_direction_label(value),
        _ => String::new(),
    }
}

fn ord_status_label(value: &str) -> String {
    match value {
        "0" => "New".into(),
        "1" => "Partially filled".into(),
        "2" => "Filled".into(),
        "3" => "Done for day".into(),
        "4" => "Canceled".into(),
        "5" => "Replaced".into(),
        "6" => "Pending Cancel".into(),
        "7" => "Stopped".into(),
        "8" => "Rejected".into(),
        "9" => "Suspended".into(),
        "A" => "Pending New".into(),
        "B" => "Calculated".into(),
        "C" => "Expired".into(),
        "D" => "Accepted for bidding".into(),
        "E" => "Pending Replace".into(),
        _ => String::new(),
    }
}

fn ord_type_label(value: &str) -> String {
    match value {
        "1" => "Market".into(),
        "2" => "Limit".into(),
        "3" => "Stop".into(),
        "4" => "Stop Limit".into(),
        "5" => "Market On Close".into(),
        "6" => "With Or Without".into(),
        "7" => "Limit Or Better".into(),
        "8" => "Limit With Or Without".into(),
        "9" => "On Basis".into(),
        "A" => "On Close".into(),
        "B" => "Limit On Close".into(),
        "C" => "Forex Market".into(),
        "D" => "Previously Quoted".into(),
        "E" => "Previously Indicated".into(),
        "F" => "Forex Limit".into(),
        "G" => "Forex Previously Quoted".into(),
        "H" => "Funari".into(),
        "I" => "Market If Touched".into(),
        "J" => "Market With Left Over as Limit".into(),
        "K" => "Previous Fund Valuation Point".into(),
        "L" => "Next Fund Valuation Point".into(),
        "M" => "Pegged".into(),
        "P" => "Pegged".into(),
        _ => String::new(),
    }
}

fn time_in_force_label(value: &str) -> String {
    match value {
        "0" => "DAY".into(),
        "1" => "GTC (Good Till Cancel)".into(),
        "2" => "OPG (At the Opening)".into(),
        "3" => "IOC (Immediate or Cancel)".into(),
        "4" => "FOK (Fill or Kill)".into(),
        "5" => "GTX (Good Till Crossing)".into(),
        "6" => "GTD (Good Till Date)".into(),
        "7" => "At the Close".into(),
        "8" => "Good Through Crossing".into(),
        "9" => "At Crossing".into(),
        _ => String::new(),
    }
}

fn exec_type_label(value: &str) -> String {
    match value {
        "0" => "New".into(),
        "1" => "Partial fill (deprecated)".into(),
        "2" => "Fill (deprecated)".into(),
        "3" => "Done for day".into(),
        "4" => "Canceled".into(),
        "5" => "Replaced".into(),
        "6" => "Pending Cancel".into(),
        "7" => "Stopped".into(),
        "8" => "Rejected".into(),
        "9" => "Suspended".into(),
        "A" => "Pending New".into(),
        "B" => "Calculated".into(),
        "C" => "Expired".into(),
        "D" => "Restated".into(),
        "E" => "Pending Replace".into(),
        "F" => "Trade".into(),
        "G" => "Trade Correct".into(),
        "H" => "Trade Cancel".into(),
        "I" => "Order Status".into(),
        _ => String::new(),
    }
}

fn handl_inst_label(value: &str) -> String {
    match value {
        "1" => "Automated execution, no intervention".into(),
        "2" => "Automated execution, intervention OK".into(),
        "3" => "Manual order".into(),
        _ => String::new(),
    }
}

fn encrypt_method_label(value: &str) -> String {
    match value {
        "0" => "None / Other".into(),
        "1" => "PKCS".into(),
        "2" => "DES".into(),
        "3" => "PKCS/DES".into(),
        "4" => "PGP/DES".into(),
        "5" => "PGP/DES-MD5".into(),
        "6" => "PEM/DES-MD5".into(),
        _ => String::new(),
    }
}

fn security_id_source_label(value: &str) -> String {
    match value {
        "1" => "CUSIP".into(),
        "2" => "SEDOL".into(),
        "3" => "QUIK".into(),
        "4" => "ISIN".into(),
        "5" => "RIC".into(),
        "6" => "ISO Currency Code".into(),
        "7" => "ISO Country Code".into(),
        "8" => "Exchange Symbol".into(),
        "9" => "Consolidated Tape Association".into(),
        _ => String::new(),
    }
}

fn exec_trans_type_label(value: &str) -> String {
    match value {
        "0" => "NEW".into(),
        "1" => "CANCEL".into(),
        "2" => "CORRECT".into(),
        "3" => "STATUS".into(),
        _ => String::new(),
    }
}

fn cxl_rej_reason_label(value: &str) -> String {
    match value {
        "0" => "Too late to cancel".into(),
        "1" => "Unknown order".into(),
        "2" => "Broker / Exchange Option".into(),
        "3" => "Order already in Pending Cancel or Pending Replace".into(),
        "4" => "Unable to process Order Mass Cancel Request".into(),
        "5" => "OrigOrdModTime did not match last TransactTime".into(),
        "6" => "Duplicate ClOrdID received".into(),
        "7" => "Price exceeds current price".into(),
        "8" => "Price exceeds current price band".into(),
        "18" => "Invalid price increment".into(),
        "99" => "Other".into(),
        _ => String::new(),
    }
}

fn ord_rej_reason_label(value: &str) -> String {
    match value {
        "0" => "Broker / Exchange option".into(),
        "1" => "Unknown symbol".into(),
        "2" => "Exchange closed".into(),
        "3" => "Order exceeds limit".into(),
        "4" => "Too late to enter".into(),
        "5" => "Unknown order".into(),
        "6" => "Duplicate Order (e.g. dupe ClOrdID)".into(),
        "7" => "Duplicate of a verbally communicated order".into(),
        "8" => "Stale order".into(),
        "9" => "Trade along required".into(),
        "10" => "Invalid Investor ID".into(),
        "11" => "Unsupported order characteristic".into(),
        "12" => "Surveillance option".into(),
        "13" => "Incorrect quantity".into(),
        "14" => "Incorrect allocated quantity".into(),
        "15" => "Unknown account(s)".into(),
        "16" => "Price exceeds current price band".into(),
        "18" => "Invalid price increment".into(),
        "19" => "Reference price not available".into(),
        "20" => "Notional value exceeds threshold".into(),
        "21" => "Algorithm risk threshold breached".into(),
        "22" => "Short sell not permitted".into(),
        "23" => "Short sell rejected (security pre-borrow)".into(),
        "24" => "Short sell rejected (account pre-borrow)".into(),
        "25" => "Insufficient credit limit".into(),
        "26" => "Exceeded clip size limit".into(),
        "27" => "Exceeded maximum notional order amount".into(),
        "28" => "Exceeded DV01/PV01 limit".into(),
        "29" => "Exceeded CS01 limit".into(),
        "99" => "Other".into(),
        _ => String::new(),
    }
}

fn session_reject_reason_label(value: &str) -> String {
    match value {
        "0" => "Invalid Tag Number".into(),
        "1" => "Required Tag Missing".into(),
        "2" => "Tag not defined for this message type".into(),
        "3" => "Undefined tag".into(),
        "4" => "Tag specified without a value".into(),
        "5" => "Value is incorrect (out of range) for this tag".into(),
        "6" => "Incorrect data format for value".into(),
        "7" => "Decryption problem".into(),
        "8" => "Signature problem".into(),
        "9" => "CompID problem".into(),
        "10" => "SendingTime Accuracy Problem".into(),
        "11" => "Invalid MsgType".into(),
        "12" => "XML Validation Error".into(),
        "13" => "Tag appears more than once".into(),
        "14" => "Tag specified out of required order".into(),
        "15" => "Repeating group fields out of order".into(),
        "16" => "Incorrect NumInGroup count for repeating group".into(),
        "17" => "Non Data value includes field delimiter".into(),
        "18" => "Invalid/Unsupported Application Version".into(),
        "99" => "Other".into(),
        _ => String::new(),
    }
}

fn position_effect_label(value: &str) -> String {
    match value {
        "O" => "Open".into(),
        "C" => "Close".into(),
        "F" => "FIFO".into(),
        "R" => "Rolled".into(),
        _ => String::new(),
    }
}

fn security_type_label(value: &str) -> String {
    match value {
        "ABS" => "Asset-backed securities".into(),
        "AMENDED" => "Amended & Restated".into(),
        "AN" => "Anticipation Notes".into(),
        "BA" => "Bankers Acceptance".into(),
        "BOND" => "Bond".into(),
        "BRADY" => "Brady Bond".into(),
        "BRIDGE" => "Bridge Loan".into(),
        "CD" => "Certificate of Deposit".into(),
        "CMBS" => "CMBS".into(),
        "COFO" => "Certificate Of Obligation".into(),
        "COFP" => "Certificate of Participation".into(),
        "CORP" => "Corporate Bond".into(),
        "CP" => "Commercial Paper".into(),
        "CPP" => "Corporate Private Placement".into(),
        "CS" => "Common Stock".into(),
        "DEFLTED" => "Defaulted".into(),
        "DINP" => "Debtor in Possession".into(),
        "DN" => "Deposit Notes".into(),
        "DUAL" => "Dual Currency".into(),
        "EUCD" => "Euro Certificate of Deposit".into(),
        "EUCORP" => "Euro Corporate Bond".into(),
        "EUSOV" => "Euro Sovereign".into(),
        "EUSUPRA" => "Euro Supranational Coupons".into(),
        "FAC" => "Federal Agency Coupon".into(),
        "FAD" => "Federal Agency Discount Note".into(),
        "FOR" => "Foreign Exchange Contract".into(),
        "FUT" => "Future".into(),
        "GIC" => "Guaranteed Investment Contract".into(),
        "GOVT" => "Government".into(),
        "IET" => "IOETTE".into(),
        "LOFC" => "Letter of Credit".into(),
        "LQN" => "Liquidity Note".into(),
        "MATURED" => "Matured".into(),
        "MF" => "Mutual Fund".into(),
        "MBS" => "Mortgage-backed securities".into(),
        "MIO" => "Mortgage Interest Only".into(),
        "MPO" => "Mortgage Principal Only".into(),
        "MPP" => "Mortgage Private Placement".into(),
        "MPT" => "Miscellaneous Pass-through".into(),
        "MTN" => "Medium Term Notes".into(),
        "MUNIC" => "Municipal".into(),
        "NONE" => "No Security Type".into(),
        "OPT" => "Option".into(),
        "PEF" => "Private Export Funding".into(),
        "PFAND" => "Pfandbriefe".into(),
        "PS" => "Preferred Stock".into(),
        "REPO" => "Repurchase".into(),
        "RETIRED" => "Retired".into(),
        "REV" => "Revenue Bonds".into(),
        "RVLV" => "Revolver Loan".into(),
        "RVLVTRM" => "Revolver/Term Loan".into(),
        "SECLOAN" => "Secured Loan".into(),
        "SLE" => "Sale Leaseback".into(),
        "SLQ" => "Student Loan Marketing Assoc".into(),
        "STN" => "Short Term Loan Note".into(),
        "STRUCT" => "Structured Notes".into(),
        "SUPRA" => "USD Supranational Coupons".into(),
        "SVERN" => "Sovereign".into(),
        "TAN" => "Tax Anticipation Note".into(),
        "TAXA" => "Tax Allocation".into(),
        "TBD" => "To Be Announced".into(),
        "TECP" => "Tax Exempt Commercial Paper".into(),
        "TRAN" => "Transmission".into(),
        "TERM" => "Term Loan".into(),
        "UST" => "US Treasury".into(),
        "USTB" => "US Treasury Bill".into(),
        "WAR" => "Warrant".into(),
        "WITHDRN" => "Withdrawn".into(),
        "WOF" => "Wellesley Off Shore Fund".into(),
        _ => String::new(),
    }
}

fn subscription_request_type_label(value: &str) -> String {
    match value {
        "0" => "Snapshot".into(),
        "1" => "Snapshot + Updates (Subscribe)".into(),
        "2" => "Disable previous Snapshot + Update (Unsubscribe)".into(),
        _ => String::new(),
    }
}

fn md_entry_type_label(value: &str) -> String {
    match value {
        "0" => "Bid".into(),
        "1" => "Offer".into(),
        "2" => "Trade".into(),
        "3" => "Index value".into(),
        "4" => "Opening price".into(),
        "5" => "Closing price".into(),
        "6" => "Settlement price".into(),
        "7" => "Trading session high price".into(),
        "8" => "Trading session low price".into(),
        "9" => "Volume Weighted Average Price".into(),
        "A" => "Imbalance".into(),
        "B" => "Trade volume".into(),
        "C" => "Open interest".into(),
        "D" => "Composite underlying price".into(),
        "H" => "Mid-price".into(),
        "J" => "Empty book".into(),
        "Q" => "Auction clearing price".into(),
        "W" => "Fixing price".into(),
        "t" => "Time Weighted Average Price".into(),
        _ => String::new(),
    }
}

fn tick_direction_label(value: &str) -> String {
    match value {
        "0" => "Plus Tick".into(),
        "1" => "Zero-Plus Tick".into(),
        "2" => "Minus Tick".into(),
        "3" => "Zero-Minus Tick".into(),
        _ => String::new(),
    }
}

fn md_update_action_label(value: &str) -> String {
    match value {
        "0" => "New".into(),
        "1" => "Change".into(),
        "2" => "Delete".into(),
        "3" => "Delete Thru".into(),
        "4" => "Delete From".into(),
        "5" => "Overlay".into(),
        _ => String::new(),
    }
}

fn exec_restatement_reason_label(value: &str) -> String {
    match value {
        "0" => "GT corporate action".into(),
        "1" => "GT renewal / restatement (no corporate action)".into(),
        "2" => "Verbal change".into(),
        "3" => "Repricing of order".into(),
        "6" => "Cancel on Trading Halt".into(),
        "7" => "Cancel on System Failure".into(),
        "9" => "Canceled, not best".into(),
        "12" => "Cancel On Connection Loss".into(),
        "13" => "Cancel On Logout".into(),
        "99" => "Other".into(),
        _ => String::new(),
    }
}

fn cxl_rej_response_to_label(value: &str) -> String {
    match value {
        "1" => "Order Cancel Request (35=F)".into(),
        "2" => "Order Cancel/Replace Request (35=G)".into(),
        _ => String::new(),
    }
}

fn product_label(value: &str) -> String {
    match value {
        "1" => "AGENCY".into(),
        "2" => "COMMODITY".into(),
        "3" => "CORPORATE".into(),
        "4" => "CURRENCY".into(),
        "5" => "EQUITY".into(),
        "6" => "GOVERNMENT".into(),
        "7" => "INDEX".into(),
        "8" => "LOAN".into(),
        "9" => "MONEYMARKET".into(),
        "10" => "MORTGAGE".into(),
        "11" => "MUNICIPAL".into(),
        "12" => "OTHER".into(),
        "13" => "FINANCING".into(),
        _ => String::new(),
    }
}

fn order_capacity_label(value: &str) -> String {
    match value {
        "A" => "Agency".into(),
        "G" => "Proprietary".into(),
        "I" => "Individual".into(),
        "P" => "Principal".into(),
        "R" => "Riskless Principal".into(),
        "W" => "Agent for Other Member".into(),
        "M" => "Mixed Capacity".into(),
        _ => String::new(),
    }
}

fn put_or_call_label(value: &str) -> String {
    match value {
        "0" => "Put".into(),
        "1" => "Call".into(),
        _ => String::new(),
    }
}

fn msg_direction_label(value: &str) -> String {
    match value {
        "S" => "Send".into(),
        "R" => "Receive".into(),
        _ => String::new(),
    }
}

fn ioi_trans_type_label(value: &str) -> String {
    match value {
        "N" => "New".into(),
        "C" => "Cancel".into(),
        "R" => "Replace".into(),
        _ => String::new(),
    }
}

fn last_capacity_label(value: &str) -> String {
    match value {
        "1" => "Agent".into(),
        "2" => "Cross as agent".into(),
        "3" => "Cross as principal".into(),
        "4" => "Principal".into(),
        "5" => "Riskless principal".into(),
        _ => String::new(),
    }
}

fn settl_type_label(value: &str) -> String {
    match value {
        "0" => "Regular / FX Spot (T+1 or T+2)".into(),
        "1" => "Cash (TOD / T+0)".into(),
        "2" => "Next Day (TOM / T+1)".into(),
        "3" => "T+2".into(),
        "4" => "T+3".into(),
        "5" => "T+4".into(),
        "6" => "Future".into(),
        "7" => "When And If Issued".into(),
        "8" => "Sellers Option".into(),
        "9" => "T+5".into(),
        "B" => "Broken date".into(),
        "C" => "FX Spot Next (Spot+1)".into(),
        _ => String::new(),
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
pub fn tag_badge_class(tag: &str) -> &'static str {
    match tag {
        "8" | "9" | "10" | "34" => "badge-slate",
        "35" => "badge-orange",
        "49" | "56" => "badge-blue",
        "52" | "60" => "badge-teal",
        "55" | "48" | "22" => "badge-purple",
        "54" | "38" | "44" | "40" => "badge-green",
        "150" | "39" | "37" | "17" => "badge-yellow",
        _ => "badge-slate",
    }
}

/// Returns `true` for tags that are considered "common" header fields.
pub fn is_common_tag(tag: &str) -> bool {
    matches!(tag, "8" | "9" | "10" | "34" | "49" | "56" | "52")
}
