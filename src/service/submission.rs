use std::collections::HashMap;

use cardano_multiplatform_lib::{plutus::{PlutusData, PlutusList, ConstrPlutusData, ExUnitPrices, self}, ledger::{common::{value::{Int, BigInt, BigNum, Value}, utxo::TransactionUnspentOutput}, alonzo::{fees::LinearFee, self}}, builders::tx_builder::{TransactionBuilderConfigBuilder, TransactionBuilder}, UnitInterval, chain_crypto::Ed25519, crypto::{PrivateKey, Bip32PrivateKey, TransactionHash}, address::{StakeCredential, Address}, TransactionInput, error::JsError, TransactionOutput, genesis::network_info::plutus_alonzo_cost_models};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

use crate::{ service::proof_of_work::get_difficulty, model::{datum_submission::{self, accept, reject, get_unconfirmed, DatumSubmission, get_newest_confirmed_datum}, proof_of_work::{self, get_by_time_range}, miner::get_miner_by_pkh, payouts::create_payout}, routes::hashrate::estimate_hashrate, address::pkh_from_address};

use super::block::{Block, KupoUtxo};

#[derive(Debug)]
pub enum SubmissionError {
    DatabaseError(sqlx::Error),
    JsError(JsError),
    ReqwestError(reqwest::Error)
}

impl From<sqlx::Error> for SubmissionError {
    fn from(err: sqlx::Error) -> Self {
        SubmissionError::DatabaseError(err)
    }
}

impl From<JsError> for SubmissionError {
    fn from(err: JsError) -> Self {
        SubmissionError::JsError(err)
    }
}

impl From<reqwest::Error> for SubmissionError {
    fn from(err: reqwest::Error) -> Self {
        SubmissionError::ReqwestError(err)
    }
}


const TUNA_VALIDATOR_CODE_MAINNET: &str = "590f86590f830100003323232323232323232323222253330083370e9000180380089929998049919299980599b87480080084c8c8c8c94ccc03ccdc3a4000601c0022646464646464646464646464646464646464646464646464646464a66605466e1d2002302900313232533302c3370e900118158048991929998172999817199817002a504a22a66605c012266e24cdc0801800a4181f82a294052809929998179981280e919baf3302c302e001480000ac4c8c8c94ccc0d4c0e00084c8c8c8c8c94ccc0dccdc3a4008606c00226464a6660726464a66607c608200426464a66607a66e3c009221096c6f72642074756e610013370e00290010a50375a607c0046eb8c0f000458c0fc004c8c94ccc0eccdc3a4004002297adef6c601323756608200260720046072002646600200203a44a66607c0022980103d87a8000132323232533303f3371e05e004266e95200033043374c00297ae0133006006003375660800066eb8c0f8008c108008c10000454ccc0e4c8c8c94ccc0fcc1080084c8c8c8c94ccc100cdc7802245001325333044304700213232533304353330433371e00a066266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c110008dd718210008b182280089929998221823802099192999821a99982199b8f00703313370e00290010a5013371e004911096c6f72642074756e610014a06eb4c110008dd718210008b18228019bab3041004375c607e0066eacc0fc010dd7181e8018b18200009820003181f0028991919baf0010033374a90001981f26010100003303e37520166607c9810105003303e4c10319ffff003303e4c10100003303e37500186607c9810100003303e4c10180004bd7019299981d19b87480000044c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc134c140008526163758609c002609c004609800260980046eb4c128004c128008dd6982400098240011bad30460013046002375a608800260880046eb8c108004c108008dd69820000981c0010b181c0008b0b181e800981a8008b181d800981d8011bab303900130390013030001163036001323300100101c22533303500114bd7009919299981a19baf3303030323303030320024800120003374a90011981c1ba90244bd7009981c00119802002000899802002000981c801181b8009919191801000980080111b9200137660542c66e00cdc199b810030014801000458dd6981900098150048b1bad30300013028003163370e900118151baa302e001302e002302c00130240053370e900118131baa302a001302a00230280013020003302600130260023024001301c002323300100100622533302200114bd6f7b630099191919299981199b8f489000021003133027337606ea4008dd3000998030030019bab3024003375c6044004604c0046048002604200260420026040002603e0046eacc074004c074004c070008dd6180d000980d000980c8011bac3017001300f005375c602a002601a0022c6026002602600460220026012008264646464a66601e66e1d2000300e00113232323300837586601c602000c9000119baf3300f30113300f30113300f301100148009200048000008cdd2a40046602a6ea40052f5c06eb8c054004c03400458c04c004c04c008c044004c02401088c8cc00400400c894ccc04400452809919299980818028010a51133004004001301500230130013008003149858c94ccc024cdc3a40000022a666018600e0062930b0a99980499b874800800454ccc030c01c00c5261616300700213223232533300c32323232323232323232323232323232323232533301f3370e9001180f0008991919191919191919191919191919191919191919299981a19b8748008c0cc0044c8c8c8c94ccc0ecc0f80084c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc124cdc3a4004609000626464a66609666e1d2002304a0091323232533304e533304e33304e0064a094454ccc1380284cdc499b8100400248303f0545280a50132323232325333053533305333710084002294454ccc14ccdc3800821099b8800204014a0264a6660a866e1cc8c8c94ccc15ccdc3a4004002290000991bad305d001305500230550013253330563370e90010008a6103d87a800013232323300100100222533305d00114c103d87a8000132323232533305e3371e911096c6f72642074756e610000213374a9000198311ba80014bd700998030030019bad305f003375c60ba00460c200460be0026eacc170004c150008c150004cc00408807d2002132325333059305c002132323232533305a533305a3371e00891010454554e410013370e006002294054ccc168c8c8c94ccc180c18c0084c8c8c8c94ccc184cdc7802245001325333065306800213232533306453330643371e00a05e266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c194008dd718318008b183300089929998329834002099192999832299983219b8f00702f13370e00290010a5013371e004911096c6f72642074756e610014a06eb4c194008dd718318008b18330019bab3062004375c60c00066eacc180010dd7182f0018b18308009830810982f8100a99982d19b8748010c1640784c8c94ccc170cdc3a400060b6002264646464646464646464646464646464a6660de60e4004264a6660daa6660daa6660da66e1ccdc3030241803e9000099b8848000180528099191919191919299983a19b87001013153330743370e004022266e1d200000f14a02940dd6983a8011bad3073001333300505e060002001375a60e40046eb4c1c00054ccc1b94ccc1b8cdc4a401066e0d2080a0c88109001133710900019b8648202832204240045280a5ef6c601010100010104001533306e533306e33712900419b8300148202832204244cdc42400066e180052080a0c8810914a0297bdb181010400010101001337606ea0005301051a48190800003370266e001600801584c94ccc1b8cdc382e8068a99983719b8705b00b13370e002012294052819b81337000b00400ac2a6660da66e1c01808054ccc1b54ccc1b4cdc399b80060480080404cdc780700f0a501533306d337126e34dd98022410010266ebcdd3999999111119199980080082c8018011111191919299983ca99983c99b8800100b14a22a6660f266e1c02c0044cdc40050010a501533307c00613307d00c3307d00c33330070074bd70001000899299983e80089983f0069983f006999980400425eb8000c0084c8cc1fc038cc1fc038cccc02402400401000cc20004004c1fc0184c8cccc00400401c0180148888c8c8c94ccc200054ccc20004cdc40008090a5115333080013370e024002266e200440085280a99984180803099842008099999803803a5eb800080044c8cc21404050cccc02002000400c008c218040184018dd69840808011bad307f0013333011002001480092004375a60f40046eb4c1e0004cccc028008005200248020dd480f00d80e02d02e1ba7002161616162222323253330723370e66e0c009208080084800054ccc1c8cdc4a40f800a297bdb18103191000000102183e001337606ea0008dd419b800054800854ccc1c8cdc42400066e0c00520808008153330723371200a90020a5ef6c6010319ffff00010102001337606ea0cdc1800a40406ea0cdc0802a4004266ec0dd40009ba800533706002901019b833370466e08011202000200116375860e000260e000460dc00260dc0046eb4c1b0004c1b0008dd6983500098350011bad30680013068002375a60cc00260cc0046eb8c190004c190008dd69831000982d0008b1830000982c00f0b0b0b299982c99b88480e80045200013370690406457d0129991919180080091299982e99b89480280044cdc1240806600400466e04005200a13003001300100122533305b3371200290000a4004266e08cc008008cdc0800a4004900200099b8304b482834464dd6982c8011bae305700116305a001323253330563370e90010008a5eb7bdb1804c8dd5982e000982a001182a0009980081380f8b11191980080080191299982d0008a6103d87a8000132323232533305b3371e00e004266e9520003305f374c00297ae0133006006003375660b80066eb8c168008c178008c17000458dd6982a0011bad3052001323253330523370e66e180092004480004c8cdd81ba8001375000666e00cdc119b8e0030014820010cdc700199b80001480084c8cdd81ba8001375000666e00cdc019b823371c00600290402019b823371c00666e00005200248080cdc199b8e0033370000290022404066e0c005200432330010014800088c94ccc14ccdc3800a4000266e012004330030033370000490010a99982999b880014808052002148000cdc70018009919191801000980080111b92001376600266e952000330523752086660a46ea0104cc148dd481f998291ba803d330523750076660a46ea00e52f5c02c66e00cdc199b8100300148010004dd6982880098248048b1bad304f0013047003163370e900118249baa304d001304d002304b00130430053370e900118229baa304900130490023047001303f003304500130450023043001303b011304100130410023756607e002607e002606c0022c6078002646600200202444a666076002297ae013232533303a3375e6606c6070004900000509981f00119802002000899802002000981f801181e8009bae303a0013032001163302f303100348000dd5981b800981b801181a800981680099299981799b8748000c0b80044c8c8cc0b4c0bc00520023035001302d00116323300100100d22533303300114c0103d87a80001323253330323375e6605c60600049000009099ba548000cc0d80092f5c0266008008002606e004606a002646600200200c44a666064002297adef6c6013232323253330333371e911000021003133037337606ea4008dd3000998030030019bab3034003375c6064004606c0046068002606200260620026060002605e0046eacc0b4004c0b4004c0b0008dd61815000981500098148011bac3027001301f0053025001301d0011630230013023002302100130190123758603e002603e002603c0046eb4c070004c070008dd6980d000980d0011bad30180013018002375a602c002602c0046eb8c050004c050008dd6980900098050030a4c2c6eb800cc94ccc02ccdc3a4000002264646464646464646464646464646464a66603c60420042930b1bac301f001301f002301d001301d002375a603600260360046eb4c064004c064008dd6980b800980b8011bad30150013015002375c602600260260046eb4c044004c02401458c024010c034c018004cc0040052000222233330073370e0020060184666600a00a66e000112002300e001002002230053754002460066ea80055cd2ab9d5573caae7d5d02ba157449812bd8799fd8799f582021fc8e4f33ca92e38f78f9bbd84cef1c037b15a86665ddba4528c7ecbc60ac90ff00ff0001";
const TUNA_VALIDATOR_CODE_PREVIEW: &str = "590f86590f830100003323232323232323232323222253330083370e9000180380089929998049919299980599b87480080084c8c8c8c94ccc03ccdc3a4000601c0022646464646464646464646464646464646464646464646464646464a66605466e1d2002302900313232533302c3370e900118158048991929998172999817199817002a504a22a66605c012266e24cdc0801800a4181f82a294052809929998179981280e919baf3302c302e001480000ac4c8c8c94ccc0d4c0e00084c8c8c8c8c94ccc0dccdc3a4008606c00226464a6660726464a66607c608200426464a66607a66e3c009221096c6f72642074756e610013370e00290010a50375a607c0046eb8c0f000458c0fc004c8c94ccc0eccdc3a4004002297adef6c601323756608200260720046072002646600200203a44a66607c0022980103d87a8000132323232533303f3371e05e004266e95200033043374c00297ae0133006006003375660800066eb8c0f8008c108008c10000454ccc0e4c8c8c94ccc0fcc1080084c8c8c8c94ccc100cdc7802245001325333044304700213232533304353330433371e00a066266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c110008dd718210008b182280089929998221823802099192999821a99982199b8f00703313370e00290010a5013371e004911096c6f72642074756e610014a06eb4c110008dd718210008b18228019bab3041004375c607e0066eacc0fc010dd7181e8018b18200009820003181f0028991919baf0010033374a90001981f26010100003303e37520166607c9810105003303e4c10319ffff003303e4c10100003303e37500186607c9810100003303e4c10180004bd7019299981d19b87480000044c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc134c140008526163758609c002609c004609800260980046eb4c128004c128008dd6982400098240011bad30460013046002375a608800260880046eb8c108004c108008dd69820000981c0010b181c0008b0b181e800981a8008b181d800981d8011bab303900130390013030001163036001323300100101c22533303500114bd7009919299981a19baf3303030323303030320024800120003374a90011981c1ba90244bd7009981c00119802002000899802002000981c801181b8009919191801000980080111b9200137660542c66e00cdc199b810030014801000458dd6981900098150048b1bad30300013028003163370e900118151baa302e001302e002302c00130240053370e900118131baa302a001302a00230280013020003302600130260023024001301c002323300100100622533302200114bd6f7b630099191919299981199b8f489000021003133027337606ea4008dd3000998030030019bab3024003375c6044004604c0046048002604200260420026040002603e0046eacc074004c074004c070008dd6180d000980d000980c8011bac3017001300f005375c602a002601a0022c6026002602600460220026012008264646464a66601e66e1d2000300e00113232323300837586601c602000c9000119baf3300f30113300f30113300f301100148009200048000008cdd2a40046602a6ea40052f5c06eb8c054004c03400458c04c004c04c008c044004c02401088c8cc00400400c894ccc04400452809919299980818028010a51133004004001301500230130013008003149858c94ccc024cdc3a40000022a666018600e0062930b0a99980499b874800800454ccc030c01c00c5261616300700213223232533300c32323232323232323232323232323232323232533301f3370e9001180f0008991919191919191919191919191919191919191919299981a19b8748008c0cc0044c8c8c8c94ccc0ecc0f80084c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc124cdc3a4004609000626464a66609666e1d2002304a0091323232533304e533304e33304e0064a094454ccc1380284cdc499b8100400248303f0545280a50132323232325333053533305333710084002294454ccc14ccdc3800821099b8800204014a0264a6660a866e1cc8c8c94ccc15ccdc3a4004002290000991bad305d001305500230550013253330563370e90010008a6103d87a800013232323300100100222533305d00114c103d87a8000132323232533305e3371e911096c6f72642074756e610000213374a9000198311ba80014bd700998030030019bad305f003375c60ba00460c200460be0026eacc170004c150008c150004cc00408807d2002132325333059305c002132323232533305a533305a3371e00891010454554e410013370e006002294054ccc168c8c8c94ccc180c18c0084c8c8c8c94ccc184cdc7802245001325333065306800213232533306453330643371e00a05e266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c194008dd718318008b183300089929998329834002099192999832299983219b8f00702f13370e00290010a5013371e004911096c6f72642074756e610014a06eb4c194008dd718318008b18330019bab3062004375c60c00066eacc180010dd7182f0018b18308009830810982f8100a99982d19b8748010c1640784c8c94ccc170cdc3a400060b6002264646464646464646464646464646464a6660de60e4004264a6660daa6660daa6660da66e1ccdc3030241803e9000099b8848000180528099191919191919299983a19b87001013153330743370e004022266e1d200000f14a02940dd6983a8011bad3073001333300505e060002001375a60e40046eb4c1c00054ccc1b94ccc1b8cdc4a401066e0d2080a0c88109001133710900019b8648202832204240045280a5ef6c601010100010104001533306e533306e33712900419b8300148202832204244cdc42400066e180052080a0c8810914a0297bdb181010400010101001337606ea0005301051a48190800003370266e001600801584c94ccc1b8cdc382e8068a99983719b8705b00b13370e002012294052819b81337000b00400ac2a6660da66e1c01808054ccc1b54ccc1b4cdc399b80060480080404cdc780700f0a501533306d337126e34dd98022410010266ebcdd3999999111119199980080082c8018011111191919299983ca99983c99b8800100b14a22a6660f266e1c02c0044cdc40050010a501533307c00613307d00c3307d00c33330070074bd70001000899299983e80089983f0069983f006999980400425eb8000c0084c8cc1fc038cc1fc038cccc02402400401000cc20004004c1fc0184c8cccc00400401c0180148888c8c8c94ccc200054ccc20004cdc40008090a5115333080013370e024002266e200440085280a99984180803099842008099999803803a5eb800080044c8cc21404050cccc02002000400c008c218040184018dd69840808011bad307f0013333011002001480092004375a60f40046eb4c1e0004cccc028008005200248020dd480f00d80e02d02e1ba7002161616162222323253330723370e66e0c009208080084800054ccc1c8cdc4a40f800a297bdb18103191000000102183e001337606ea0008dd419b800054800854ccc1c8cdc42400066e0c00520808008153330723371200a90020a5ef6c6010319ffff00010102001337606ea0cdc1800a40406ea0cdc0802a4004266ec0dd40009ba800533706002901019b833370466e08011202000200116375860e000260e000460dc00260dc0046eb4c1b0004c1b0008dd6983500098350011bad30680013068002375a60cc00260cc0046eb8c190004c190008dd69831000982d0008b1830000982c00f0b0b0b299982c99b88480e80045200013370690406457d0129991919180080091299982e99b89480280044cdc1240806600400466e04005200a13003001300100122533305b3371200290000a4004266e08cc008008cdc0800a4004900200099b8304b482834464dd6982c8011bae305700116305a001323253330563370e90010008a5eb7bdb1804c8dd5982e000982a001182a0009980081380f8b11191980080080191299982d0008a6103d87a8000132323232533305b3371e00e004266e9520003305f374c00297ae0133006006003375660b80066eb8c168008c178008c17000458dd6982a0011bad3052001323253330523370e66e180092004480004c8cdd81ba8001375000666e00cdc119b8e0030014820010cdc700199b80001480084c8cdd81ba8001375000666e00cdc019b823371c00600290402019b823371c00666e00005200248080cdc199b8e0033370000290022404066e0c005200432330010014800088c94ccc14ccdc3800a4000266e012004330030033370000490010a99982999b880014808052002148000cdc70018009919191801000980080111b92001376600266e952000330523752086660a46ea0104cc148dd481f998291ba803d330523750076660a46ea00e52f5c02c66e00cdc199b8100300148010004dd6982880098248048b1bad304f0013047003163370e900118249baa304d001304d002304b00130430053370e900118229baa304900130490023047001303f003304500130450023043001303b011304100130410023756607e002607e002606c0022c6078002646600200202444a666076002297ae013232533303a3375e6606c6070004900000509981f00119802002000899802002000981f801181e8009bae303a0013032001163302f303100348000dd5981b800981b801181a800981680099299981799b8748000c0b80044c8c8cc0b4c0bc00520023035001302d00116323300100100d22533303300114c0103d87a80001323253330323375e6605c60600049000009099ba548000cc0d80092f5c0266008008002606e004606a002646600200200c44a666064002297adef6c6013232323253330333371e911000021003133037337606ea4008dd3000998030030019bab3034003375c6064004606c0046068002606200260620026060002605e0046eacc0b4004c0b4004c0b0008dd61815000981500098148011bac3027001301f0053025001301d0011630230013023002302100130190123758603e002603e002603c0046eb4c070004c070008dd6980d000980d0011bad30180013018002375a602c002602c0046eb8c050004c050008dd6980900098050030a4c2c6eb800cc94ccc02ccdc3a4000002264646464646464646464646464646464a66603c60420042930b1bac301f001301f002301d001301d002375a603600260360046eb4c064004c064008dd6980b800980b8011bad30150013015002375c602600260260046eb4c044004c02401458c024010c034c018004cc0040052000222233330073370e0020060184666600a00a66e000112002300e001002002230053754002460066ea80055cd2ab9d5573caae7d5d02ba157449812bd8799fd8799f5820580c37415cf5b98da27f845ed853f2e4fda0034c1441c99eb3a7f333483ce99dff02ff0001";
const TUNA_VALIDATOR_HASH_MAINNET: &str = "279f842c33eed9054b9e3c70cd6a3b32298259c24b78b895cb41d91a";
const TUNA_VALIDATOR_HASH_PREVIEW: &str = "502fbfbdafc7ddada9c335bd1440781e5445d08bada77dc2032866a6";
const TUNA_PREVIEW_ADDRESS: &str = "addr_test1wpgzl0aa4lramtdfcv6m69zq0q09g3ws3wk6wlwzqv5xdfsdcf2qa";
const TUNA_MAINNET_ADDRESS: &str = "";
const EPOCH_NUMBER: u64 = 2016;
const EPOCH_TARGET: u64 = 1_209_600;
const PADDING: u64 = 16;
pub const ON_CHAIN_HALF_TIME_RANGE: u64 = 90;

const TUNA_PER_DATUM: usize = 5_000_000_000;
#[derive(Debug, Serialize)]
pub struct DenoSubmission {
    nonce: String,
    sha: String,
    current_block: Block,
    new_zeroes: i64,
    new_difficulty: i64,
    miner_payments: HashMap<String, usize>,  // <Address, Payment>
}
#[derive(Deserialize)]
pub struct DenoSubmissionResponse {
    tx_hash: String,
    message: String,
}

pub async fn submit(
    pool: &SqlitePool,
    current_block: &Block,
    sha: &[u8],
    nonce: &[u8]
) -> Result<(), SubmissionError> {
    let new_diff_data = get_difficulty(sha);

    let default_fee: i64 = 25000000;
    let pool_fixed_fee: i64 = std::env::var("POOL_FIXED_FEE")
        .map(|s| s.parse().unwrap_or(default_fee))
        .unwrap_or(default_fee);

    let total_payout = TUNA_PER_DATUM - pool_fixed_fee as usize;

    let maybe_last_paid_datum = get_newest_confirmed_datum(pool).await?;

    let start_time = match maybe_last_paid_datum {
        Some(last_paid_datum) => {
            let maybe_last_confirmed = last_paid_datum.confirmed_at;
            if maybe_last_confirmed.is_some() {
                maybe_last_confirmed.unwrap()
            } else {
                proof_of_work::get_oldest(pool)
                .await
                .unwrap().unwrap().created_at
            }
        },
        None => {
            // if no datum has ever been paid before, start from the first proof of work
            proof_of_work::get_oldest(pool)
                .await
                .unwrap().unwrap().created_at  // invariant upheld by the fact that we have a datum
        }
    };
    
    let end_time = Utc::now().naive_utc();
    
    let proofs = get_by_time_range(pool, None, start_time, end_time).await?;

    let estimated_hashrate_total = estimate_hashrate(&proofs, start_time, end_time);

    let mut miner_proofs: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
    for proof in &proofs {
        miner_proofs.entry(proof.miner_address.clone()).or_insert_with(Vec::new).push(proof.clone());
    }

    let mut miner_payments: HashMap<String, usize> = HashMap::new();
    for (miner_address, proofs) in &miner_proofs { 
        let miner_hashrate = estimate_hashrate(proofs.as_ref(), start_time, end_time);
        let miner_share = miner_hashrate as f64 / estimated_hashrate_total as f64;
        let miner_payment = (total_payout as f64 * miner_share) as usize;
        miner_payments.insert(miner_address.clone(), miner_payment);
    }

    let submission = DenoSubmission {
        nonce: hex::encode(nonce),
        sha: hex::encode(sha),
        current_block: current_block.clone(),
        new_difficulty: new_diff_data.difficulty_number as i64,
        new_zeroes: new_diff_data.leading_zeroes as i64,
        miner_payments: miner_payments.clone(),
    };

    let response: DenoSubmissionResponse = reqwest::Client::new()
        .post("http://localhost:22123/submit")
        .json(&submission)
        .send()
        .await?
        .json()
        .await?;

    log::info!("Submitted datum on chain in tx_hash {}", &response.tx_hash);

    datum_submission::create(
        pool, response.tx_hash.clone(), hex::encode(sha)
    ).await?;

    for (miner_address, payment) in &miner_payments {
        let Ok(pkh) = pkh_from_address(miner_address) else {
            continue;
        };
        let Some(miner) = get_miner_by_pkh(pool, &pkh).await? else {
            continue;
        };
    
        let mut tx = pool.begin().await?;
    
        create_payout(&mut tx, miner.id, *payment as i64, &response.tx_hash).await?;
    
        tx.commit().await?;
    }

    Ok(())
}

pub async fn submission_updater(pool: SqlitePool) {
    let kupo_url = std::env::var("KUPO_URL").expect("Cannot instantiate BlockService because KUPO_URL is not set.");
    let interval = 60;

    let client = reqwest::Client::new();

    loop {
        
        let unconfirmed_datums = get_unconfirmed(&pool).await;
        
        let Ok(unconfirmed) = unconfirmed_datums else {
            log::error!("Submission updater could not fetch unconfirmed.");
            return;
        };

        if !unconfirmed.is_empty() {
            for datum in unconfirmed {
                let url = format!("{}/matches/*@{}", kupo_url, datum.transaction_hash);

                let resp = client.get(&url).send().await;

                let Ok(r) = resp else {
                    log::warn!("Failed to fetch matches for transaction_id: {}", datum.transaction_hash);
                    continue;
                };
                
                let response_result: Result<Vec<KupoUtxo>, reqwest::Error> = r.json().await;
                let Ok(kupo_utxos) = response_result else {
                    log::error!("Failed to parse kupo transaction! Got {:?}", response_result);
                    continue;
                };

                let tx_hash = datum.transaction_hash.clone();
                if !kupo_utxos.is_empty() {
                    let slot_no = kupo_utxos[0].created_at.slot_no;
                    let datum = DatumSubmission {
                        confirmed_in_slot: Some(slot_no),
                        ..datum
                    };
                    let result = accept(&pool, vec![datum]).await;
                    log::info!("Permanently accepted datum at transaction {}.", tx_hash);
                    let Ok(_) = result else {
                        log::error!("Failed to accept datum with transaction_id: {}", tx_hash);
                        return;
                    };
                } else {
                    let now = Utc::now().naive_utc();
                    let age = now.signed_duration_since(datum.created_at).num_minutes();

                    if age > 2 {
                        let result = reject(&pool, vec![datum]).await;
                        let Ok(_) = result else {
                            log::error!("Failed to reject datum with transaction_id: {}", tx_hash);
                            return;
                        };
                    }
                }
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
    }
}

// Below here is a code graveyard
// Where I tried and failed to do submissions from rust

fn new_post_datum(current_block: &Block, sha: &[u8], current_time: DateTime<Utc>) -> PlutusData {
    let new_time_off_chain = current_time.timestamp() as u64;
    let new_time_on_chain = new_time_off_chain + ON_CHAIN_HALF_TIME_RANGE;

    let interlink = &current_block.interlink;

    let mut new_block = Block {
        block_number: current_block.block_number + 1,
        current_hash: hex::encode(sha).into(),
        leading_zeroes: current_block.leading_zeroes,
        difficulty_number: current_block.difficulty_number,
        epoch_time: current_block.epoch_time,
        current_time: new_time_on_chain,
        extra: vec!(),
        interlink: interlink.clone(),
        output_index: current_block.output_index,
        transaction_id: current_block.transaction_id.clone()
    };

    if current_block.block_number as u64 % EPOCH_NUMBER == 0 {
        new_block.epoch_time = current_block.epoch_time + new_block.current_time - current_block.current_time;

        change_difficulty(&mut new_block);
    } else {
        new_block.epoch_time = current_block.epoch_time + new_block.current_time - current_block.current_time;
    }

    let difficulty_number = new_block.difficulty_number;
    calculate_interlink(&mut new_block, difficulty_number, current_block.leading_zeroes);
    let mut post_datum_fields = PlutusList::new();
    let block_number_field = PlutusData::new_integer(&BigInt::from(new_block.block_number));
    let hash_field = PlutusData::new_bytes(hex::decode(&new_block.current_hash).unwrap_or_default());
    let zeroes_field = PlutusData::new_integer(&BigInt::from(new_block.leading_zeroes));
    let difficulty_field = PlutusData::new_integer(&BigInt::from(new_block.difficulty_number));
    let epoch_field = PlutusData::new_integer(&BigInt::from(new_block.epoch_time));
    let time_field = PlutusData::new_integer(&BigInt::from(new_time_off_chain + 90000));
    let extra_field = PlutusData::new_integer(&BigInt::from(0));

    let mut interlink_list = PlutusList::new();
    for i in 0..new_block.interlink.len() {
        interlink_list.add(&PlutusData::new_bytes(new_block.interlink[i].clone()))
    }
    let interlink_field = PlutusData::new_list(&interlink_list);

    post_datum_fields.add(&block_number_field);
    post_datum_fields.add(&hash_field);
    post_datum_fields.add(&zeroes_field);
    post_datum_fields.add(&difficulty_field);
    post_datum_fields.add(&epoch_field);
    post_datum_fields.add(&time_field);
    post_datum_fields.add(&extra_field);
    post_datum_fields.add(&interlink_field);

    PlutusData::new_constr_plutus_data(
        &ConstrPlutusData::new(&BigNum::from(0), &post_datum_fields)
    )
}

pub fn build_transaction(
    current_block: &Block,
    sha: &[u8; 32],
) -> () {
        // TODO: Check this on startup
        let private_key_ed25519 = std::env::var("MINING_WALLET_PRIVATE_KEY").expect("Must have a MINING_WALLET_PRIVATE_KEY set!");
        let private_key = PrivateKey::from_bech32(&private_key_ed25519).unwrap();
        let cred = StakeCredential::from_keyhash(&private_key.to_public().hash());
        
        let utc: DateTime<Utc> = Utc::now();
        let new_plutus_data = new_post_datum(&current_block, sha, utc);
    
        // TODO: Don't hardcode any of this.
        // TODO: What's missing? :(
        let fee = LinearFee::new(
            &BigNum::from(44),
            &BigNum::from(155381)
        );
        let tx_builder_cfg = TransactionBuilderConfigBuilder::new()
            .fee_algo(&fee)
            .costmdls(&plutus_alonzo_cost_models())
            .max_value_size(5000)
            .max_tx_size(16384)
            .coins_per_utxo_byte(&BigNum::from(4310))
            .ex_unit_prices(&ExUnitPrices::new(
                &UnitInterval::new(&BigNum::from(721), &BigNum::from(10_000_000)),
                &UnitInterval::new(&BigNum::from(577), &BigNum::from(10_000))
            ))
            .collateral_percentage(150)
            .build()
            .unwrap();
    
        let tx_builder = TransactionBuilder::new(&tx_builder_cfg);
        
        //let current_contract_tx_hash = TransactionHash::from_hex(&current_block.transaction_id)?;
    
        // let output_to_be_spent = TransactionOutput::new(
        //     &Address::from_bech32(TUNA_PREVIEW_ADDRESS),
        //     Value::new(coin)
        // )
        // let input_to_be_spent = TransactionInput::new(
        //     &current_contract_tx_hash,
        //     &BigNum::from(current_block.output_index as u64)
        // );
        // let utxo_to_be_spent = TransactionUnspentOutput::new(input_to_be_spent)
        
}

fn calculate_interlink(new_block: &mut Block, difficulty_number: u16, leading_zeros: u8) {
    let mut b = Difficulty {
        leading_zeros: new_block.leading_zeroes as i128,
        difficulty_number: new_block.difficulty_number as i128,
    };

    let a = Difficulty {
        leading_zeros: leading_zeros as i128,
        difficulty_number: difficulty_number as i128,
    };

    let mut b_half = half_difficulty_number(b);

    let mut current_index = 0;

    while b_half.leading_zeros < a.leading_zeros
        || (b_half.leading_zeros == a.leading_zeros && b_half.difficulty_number > a.difficulty_number)
    {
        if current_index < new_block.interlink.len() {
            new_block.interlink[current_index] = new_block.current_hash.clone();
        } else {
            new_block.interlink.push(new_block.current_hash.clone());
        }

        b_half = half_difficulty_number(b_half);
        current_index += 1;
    }
}

struct Difficulty {
    leading_zeros: i128,
    difficulty_number: i128,
}

fn half_difficulty_number(a: Difficulty) -> Difficulty {
    let new_a = a.difficulty_number / 2;

    if new_a < 4096 {
        Difficulty {
            leading_zeros: a.leading_zeros + 1,
            difficulty_number: new_a * 16,
        }
    } else {
        Difficulty {
            leading_zeros: a.leading_zeros,
            difficulty_number: new_a,
        }
    }
}

fn change_difficulty(block: &mut Block) {
    let current_difficulty = block.difficulty_number as u64;
    let leading_zeros = block.leading_zeroes;
    let total_epoch_time = block.epoch_time;

    let (new_difficulty, new_leading_zeroes) =
        calculate_new_difficulty(total_epoch_time, current_difficulty, leading_zeros);

    block.difficulty_number = new_difficulty;
    block.leading_zeroes = new_leading_zeroes;
    block.epoch_time = 0;
}

fn calculate_new_difficulty(
    total_epoch_time: u64,
    current_difficulty: u64,
    leading_zeros: u8,
) -> (u16, u8) {
    let difficulty_adjustment = if EPOCH_TARGET / total_epoch_time >= 4 {
        (1, 4)
    } else if total_epoch_time / EPOCH_TARGET >= 4 {
        (4, 1)
    } else {
        (total_epoch_time, EPOCH_TARGET)
    };

    let new_padded_difficulty =
        current_difficulty * PADDING * difficulty_adjustment.0 / difficulty_adjustment.1;
    let new_difficulty = new_padded_difficulty / PADDING;

    if new_padded_difficulty / 65536 == 0 {
        if leading_zeros >= 30 {
            (4096, 60)
        } else {
            (new_padded_difficulty as u16, leading_zeros + 1)
        }
    } else if new_difficulty / 65536 > 0 {
        if leading_zeros <= 2 {
            (65535, 2)
        } else {
            ((new_difficulty / PADDING) as u16, leading_zeros - 1)
        }
    } else {
        println!("{}, {}", new_padded_difficulty / 65536, new_difficulty / 65536);
        (new_difficulty as u16, leading_zeros)
    }
}