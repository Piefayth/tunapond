(BigInt.prototype as any).toJSON = function () {
  return this.toString();
};

import { loadSync } from "https://deno.land/std@0.201.0/dotenv/mod.ts"
import { Constr, Data, Kupmios, Lucid, Network, UTxO, fromHex, fromText, toHex } from "https://deno.land/x/lucid@0.10.1/mod.ts"

loadSync({ export: true })

const delay = (ms: number | undefined) =>
  new Promise((res) => setTimeout(res, ms));

const miningWalletPrivateKey = Deno.env.get("MINING_WALLET_PRIVATE_KEY")
const network = Deno.env.get("NETWORK") as Network
const KUPO_URL = Deno.env.get("KUPO_URL")!
const OGMIOS_URL = Deno.env.get("OGMIOS_URL")!
const POOL_CONTRACT_ADDRESS = Deno.env.get("POOL_CONTRACT_ADDRESS")!
const POOL_SCRIPT_HASH = Deno.env.get("POOL_SCRIPT_HASH")!
const [POOL_OUTPUT_REF_TX, POOL_OUTPUT_REF_INDEX] = Deno.env.get("POOL_OUTPUT_REFERENCE")!.split("#")

if (!POOL_OUTPUT_REF_TX || !POOL_OUTPUT_REF_INDEX) {
  throw Error("POOL_OUTPUT_REFERENCE was malformed or otherwise invalid.")
}

const lucid = await Lucid.new(new Kupmios(KUPO_URL, OGMIOS_URL), network)
const poolScriptRef = await lucid.utxosByOutRef([{
  txHash: POOL_OUTPUT_REF_TX,
  outputIndex: Number(POOL_OUTPUT_REF_INDEX)
}])

if (poolScriptRef.length === 0) {
  throw Error("Could not boot because the reference input for the pool script could not be found. Either POOL_OUTPUT_REFERENCE is misconfigured, or the Lucid provider does not have this data available to it.")
}

lucid.selectWalletFromPrivateKey(miningWalletPrivateKey!)
const poolRewardAddress = await lucid.wallet.address()

const TUNA_VALIDATOR_CODE_MAINNET = "590f86590f830100003323232323232323232323222253330083370e9000180380089929998049919299980599b87480080084c8c8c8c94ccc03ccdc3a4000601c0022646464646464646464646464646464646464646464646464646464a66605466e1d2002302900313232533302c3370e900118158048991929998172999817199817002a504a22a66605c012266e24cdc0801800a4181f82a294052809929998179981280e919baf3302c302e001480000ac4c8c8c94ccc0d4c0e00084c8c8c8c8c94ccc0dccdc3a4008606c00226464a6660726464a66607c608200426464a66607a66e3c009221096c6f72642074756e610013370e00290010a50375a607c0046eb8c0f000458c0fc004c8c94ccc0eccdc3a4004002297adef6c601323756608200260720046072002646600200203a44a66607c0022980103d87a8000132323232533303f3371e05e004266e95200033043374c00297ae0133006006003375660800066eb8c0f8008c108008c10000454ccc0e4c8c8c94ccc0fcc1080084c8c8c8c94ccc100cdc7802245001325333044304700213232533304353330433371e00a066266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c110008dd718210008b182280089929998221823802099192999821a99982199b8f00703313370e00290010a5013371e004911096c6f72642074756e610014a06eb4c110008dd718210008b18228019bab3041004375c607e0066eacc0fc010dd7181e8018b18200009820003181f0028991919baf0010033374a90001981f26010100003303e37520166607c9810105003303e4c10319ffff003303e4c10100003303e37500186607c9810100003303e4c10180004bd7019299981d19b87480000044c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc134c140008526163758609c002609c004609800260980046eb4c128004c128008dd6982400098240011bad30460013046002375a608800260880046eb8c108004c108008dd69820000981c0010b181c0008b0b181e800981a8008b181d800981d8011bab303900130390013030001163036001323300100101c22533303500114bd7009919299981a19baf3303030323303030320024800120003374a90011981c1ba90244bd7009981c00119802002000899802002000981c801181b8009919191801000980080111b9200137660542c66e00cdc199b810030014801000458dd6981900098150048b1bad30300013028003163370e900118151baa302e001302e002302c00130240053370e900118131baa302a001302a00230280013020003302600130260023024001301c002323300100100622533302200114bd6f7b630099191919299981199b8f489000021003133027337606ea4008dd3000998030030019bab3024003375c6044004604c0046048002604200260420026040002603e0046eacc074004c074004c070008dd6180d000980d000980c8011bac3017001300f005375c602a002601a0022c6026002602600460220026012008264646464a66601e66e1d2000300e00113232323300837586601c602000c9000119baf3300f30113300f30113300f301100148009200048000008cdd2a40046602a6ea40052f5c06eb8c054004c03400458c04c004c04c008c044004c02401088c8cc00400400c894ccc04400452809919299980818028010a51133004004001301500230130013008003149858c94ccc024cdc3a40000022a666018600e0062930b0a99980499b874800800454ccc030c01c00c5261616300700213223232533300c32323232323232323232323232323232323232533301f3370e9001180f0008991919191919191919191919191919191919191919299981a19b8748008c0cc0044c8c8c8c94ccc0ecc0f80084c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc124cdc3a4004609000626464a66609666e1d2002304a0091323232533304e533304e33304e0064a094454ccc1380284cdc499b8100400248303f0545280a50132323232325333053533305333710084002294454ccc14ccdc3800821099b8800204014a0264a6660a866e1cc8c8c94ccc15ccdc3a4004002290000991bad305d001305500230550013253330563370e90010008a6103d87a800013232323300100100222533305d00114c103d87a8000132323232533305e3371e911096c6f72642074756e610000213374a9000198311ba80014bd700998030030019bad305f003375c60ba00460c200460be0026eacc170004c150008c150004cc00408807d2002132325333059305c002132323232533305a533305a3371e00891010454554e410013370e006002294054ccc168c8c8c94ccc180c18c0084c8c8c8c94ccc184cdc7802245001325333065306800213232533306453330643371e00a05e266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c194008dd718318008b183300089929998329834002099192999832299983219b8f00702f13370e00290010a5013371e004911096c6f72642074756e610014a06eb4c194008dd718318008b18330019bab3062004375c60c00066eacc180010dd7182f0018b18308009830810982f8100a99982d19b8748010c1640784c8c94ccc170cdc3a400060b6002264646464646464646464646464646464a6660de60e4004264a6660daa6660daa6660da66e1ccdc3030241803e9000099b8848000180528099191919191919299983a19b87001013153330743370e004022266e1d200000f14a02940dd6983a8011bad3073001333300505e060002001375a60e40046eb4c1c00054ccc1b94ccc1b8cdc4a401066e0d2080a0c88109001133710900019b8648202832204240045280a5ef6c601010100010104001533306e533306e33712900419b8300148202832204244cdc42400066e180052080a0c8810914a0297bdb181010400010101001337606ea0005301051a48190800003370266e001600801584c94ccc1b8cdc382e8068a99983719b8705b00b13370e002012294052819b81337000b00400ac2a6660da66e1c01808054ccc1b54ccc1b4cdc399b80060480080404cdc780700f0a501533306d337126e34dd98022410010266ebcdd3999999111119199980080082c8018011111191919299983ca99983c99b8800100b14a22a6660f266e1c02c0044cdc40050010a501533307c00613307d00c3307d00c33330070074bd70001000899299983e80089983f0069983f006999980400425eb8000c0084c8cc1fc038cc1fc038cccc02402400401000cc20004004c1fc0184c8cccc00400401c0180148888c8c8c94ccc200054ccc20004cdc40008090a5115333080013370e024002266e200440085280a99984180803099842008099999803803a5eb800080044c8cc21404050cccc02002000400c008c218040184018dd69840808011bad307f0013333011002001480092004375a60f40046eb4c1e0004cccc028008005200248020dd480f00d80e02d02e1ba7002161616162222323253330723370e66e0c009208080084800054ccc1c8cdc4a40f800a297bdb18103191000000102183e001337606ea0008dd419b800054800854ccc1c8cdc42400066e0c00520808008153330723371200a90020a5ef6c6010319ffff00010102001337606ea0cdc1800a40406ea0cdc0802a4004266ec0dd40009ba800533706002901019b833370466e08011202000200116375860e000260e000460dc00260dc0046eb4c1b0004c1b0008dd6983500098350011bad30680013068002375a60cc00260cc0046eb8c190004c190008dd69831000982d0008b1830000982c00f0b0b0b299982c99b88480e80045200013370690406457d0129991919180080091299982e99b89480280044cdc1240806600400466e04005200a13003001300100122533305b3371200290000a4004266e08cc008008cdc0800a4004900200099b8304b482834464dd6982c8011bae305700116305a001323253330563370e90010008a5eb7bdb1804c8dd5982e000982a001182a0009980081380f8b11191980080080191299982d0008a6103d87a8000132323232533305b3371e00e004266e9520003305f374c00297ae0133006006003375660b80066eb8c168008c178008c17000458dd6982a0011bad3052001323253330523370e66e180092004480004c8cdd81ba8001375000666e00cdc119b8e0030014820010cdc700199b80001480084c8cdd81ba8001375000666e00cdc019b823371c00600290402019b823371c00666e00005200248080cdc199b8e0033370000290022404066e0c005200432330010014800088c94ccc14ccdc3800a4000266e012004330030033370000490010a99982999b880014808052002148000cdc70018009919191801000980080111b92001376600266e952000330523752086660a46ea0104cc148dd481f998291ba803d330523750076660a46ea00e52f5c02c66e00cdc199b8100300148010004dd6982880098248048b1bad304f0013047003163370e900118249baa304d001304d002304b00130430053370e900118229baa304900130490023047001303f003304500130450023043001303b011304100130410023756607e002607e002606c0022c6078002646600200202444a666076002297ae013232533303a3375e6606c6070004900000509981f00119802002000899802002000981f801181e8009bae303a0013032001163302f303100348000dd5981b800981b801181a800981680099299981799b8748000c0b80044c8c8cc0b4c0bc00520023035001302d00116323300100100d22533303300114c0103d87a80001323253330323375e6605c60600049000009099ba548000cc0d80092f5c0266008008002606e004606a002646600200200c44a666064002297adef6c6013232323253330333371e911000021003133037337606ea4008dd3000998030030019bab3034003375c6064004606c0046068002606200260620026060002605e0046eacc0b4004c0b4004c0b0008dd61815000981500098148011bac3027001301f0053025001301d0011630230013023002302100130190123758603e002603e002603c0046eb4c070004c070008dd6980d000980d0011bad30180013018002375a602c002602c0046eb8c050004c050008dd6980900098050030a4c2c6eb800cc94ccc02ccdc3a4000002264646464646464646464646464646464a66603c60420042930b1bac301f001301f002301d001301d002375a603600260360046eb4c064004c064008dd6980b800980b8011bad30150013015002375c602600260260046eb4c044004c02401458c024010c034c018004cc0040052000222233330073370e0020060184666600a00a66e000112002300e001002002230053754002460066ea80055cd2ab9d5573caae7d5d02ba157449812bd8799fd8799f582021fc8e4f33ca92e38f78f9bbd84cef1c037b15a86665ddba4528c7ecbc60ac90ff00ff0001"
const TUNA_VALIDATOR_CODE_PREVIEW = "590f86590f830100003323232323232323232323222253330083370e9000180380089929998049919299980599b87480080084c8c8c8c94ccc03ccdc3a4000601c0022646464646464646464646464646464646464646464646464646464a66605466e1d2002302900313232533302c3370e900118158048991929998172999817199817002a504a22a66605c012266e24cdc0801800a4181f82a294052809929998179981280e919baf3302c302e001480000ac4c8c8c94ccc0d4c0e00084c8c8c8c8c94ccc0dccdc3a4008606c00226464a6660726464a66607c608200426464a66607a66e3c009221096c6f72642074756e610013370e00290010a50375a607c0046eb8c0f000458c0fc004c8c94ccc0eccdc3a4004002297adef6c601323756608200260720046072002646600200203a44a66607c0022980103d87a8000132323232533303f3371e05e004266e95200033043374c00297ae0133006006003375660800066eb8c0f8008c108008c10000454ccc0e4c8c8c94ccc0fcc1080084c8c8c8c94ccc100cdc7802245001325333044304700213232533304353330433371e00a066266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c110008dd718210008b182280089929998221823802099192999821a99982199b8f00703313370e00290010a5013371e004911096c6f72642074756e610014a06eb4c110008dd718210008b18228019bab3041004375c607e0066eacc0fc010dd7181e8018b18200009820003181f0028991919baf0010033374a90001981f26010100003303e37520166607c9810105003303e4c10319ffff003303e4c10100003303e37500186607c9810100003303e4c10180004bd7019299981d19b87480000044c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc134c140008526163758609c002609c004609800260980046eb4c128004c128008dd6982400098240011bad30460013046002375a608800260880046eb8c108004c108008dd69820000981c0010b181c0008b0b181e800981a8008b181d800981d8011bab303900130390013030001163036001323300100101c22533303500114bd7009919299981a19baf3303030323303030320024800120003374a90011981c1ba90244bd7009981c00119802002000899802002000981c801181b8009919191801000980080111b9200137660542c66e00cdc199b810030014801000458dd6981900098150048b1bad30300013028003163370e900118151baa302e001302e002302c00130240053370e900118131baa302a001302a00230280013020003302600130260023024001301c002323300100100622533302200114bd6f7b630099191919299981199b8f489000021003133027337606ea4008dd3000998030030019bab3024003375c6044004604c0046048002604200260420026040002603e0046eacc074004c074004c070008dd6180d000980d000980c8011bac3017001300f005375c602a002601a0022c6026002602600460220026012008264646464a66601e66e1d2000300e00113232323300837586601c602000c9000119baf3300f30113300f30113300f301100148009200048000008cdd2a40046602a6ea40052f5c06eb8c054004c03400458c04c004c04c008c044004c02401088c8cc00400400c894ccc04400452809919299980818028010a51133004004001301500230130013008003149858c94ccc024cdc3a40000022a666018600e0062930b0a99980499b874800800454ccc030c01c00c5261616300700213223232533300c32323232323232323232323232323232323232533301f3370e9001180f0008991919191919191919191919191919191919191919299981a19b8748008c0cc0044c8c8c8c94ccc0ecc0f80084c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c8c94ccc124cdc3a4004609000626464a66609666e1d2002304a0091323232533304e533304e33304e0064a094454ccc1380284cdc499b8100400248303f0545280a50132323232325333053533305333710084002294454ccc14ccdc3800821099b8800204014a0264a6660a866e1cc8c8c94ccc15ccdc3a4004002290000991bad305d001305500230550013253330563370e90010008a6103d87a800013232323300100100222533305d00114c103d87a8000132323232533305e3371e911096c6f72642074756e610000213374a9000198311ba80014bd700998030030019bad305f003375c60ba00460c200460be0026eacc170004c150008c150004cc00408807d2002132325333059305c002132323232533305a533305a3371e00891010454554e410013370e006002294054ccc168c8c8c94ccc180c18c0084c8c8c8c94ccc184cdc7802245001325333065306800213232533306453330643371e00a05e266e1c005200214a0266e3c009221096c6f72642074756e610014a06eb4c194008dd718318008b183300089929998329834002099192999832299983219b8f00702f13370e00290010a5013371e004911096c6f72642074756e610014a06eb4c194008dd718318008b18330019bab3062004375c60c00066eacc180010dd7182f0018b18308009830810982f8100a99982d19b8748010c1640784c8c94ccc170cdc3a400060b6002264646464646464646464646464646464a6660de60e4004264a6660daa6660daa6660da66e1ccdc3030241803e9000099b8848000180528099191919191919299983a19b87001013153330743370e004022266e1d200000f14a02940dd6983a8011bad3073001333300505e060002001375a60e40046eb4c1c00054ccc1b94ccc1b8cdc4a401066e0d2080a0c88109001133710900019b8648202832204240045280a5ef6c601010100010104001533306e533306e33712900419b8300148202832204244cdc42400066e180052080a0c8810914a0297bdb181010400010101001337606ea0005301051a48190800003370266e001600801584c94ccc1b8cdc382e8068a99983719b8705b00b13370e002012294052819b81337000b00400ac2a6660da66e1c01808054ccc1b54ccc1b4cdc399b80060480080404cdc780700f0a501533306d337126e34dd98022410010266ebcdd3999999111119199980080082c8018011111191919299983ca99983c99b8800100b14a22a6660f266e1c02c0044cdc40050010a501533307c00613307d00c3307d00c33330070074bd70001000899299983e80089983f0069983f006999980400425eb8000c0084c8cc1fc038cc1fc038cccc02402400401000cc20004004c1fc0184c8cccc00400401c0180148888c8c8c94ccc200054ccc20004cdc40008090a5115333080013370e024002266e200440085280a99984180803099842008099999803803a5eb800080044c8cc21404050cccc02002000400c008c218040184018dd69840808011bad307f0013333011002001480092004375a60f40046eb4c1e0004cccc028008005200248020dd480f00d80e02d02e1ba7002161616162222323253330723370e66e0c009208080084800054ccc1c8cdc4a40f800a297bdb18103191000000102183e001337606ea0008dd419b800054800854ccc1c8cdc42400066e0c00520808008153330723371200a90020a5ef6c6010319ffff00010102001337606ea0cdc1800a40406ea0cdc0802a4004266ec0dd40009ba800533706002901019b833370466e08011202000200116375860e000260e000460dc00260dc0046eb4c1b0004c1b0008dd6983500098350011bad30680013068002375a60cc00260cc0046eb8c190004c190008dd69831000982d0008b1830000982c00f0b0b0b299982c99b88480e80045200013370690406457d0129991919180080091299982e99b89480280044cdc1240806600400466e04005200a13003001300100122533305b3371200290000a4004266e08cc008008cdc0800a4004900200099b8304b482834464dd6982c8011bae305700116305a001323253330563370e90010008a5eb7bdb1804c8dd5982e000982a001182a0009980081380f8b11191980080080191299982d0008a6103d87a8000132323232533305b3371e00e004266e9520003305f374c00297ae0133006006003375660b80066eb8c168008c178008c17000458dd6982a0011bad3052001323253330523370e66e180092004480004c8cdd81ba8001375000666e00cdc119b8e0030014820010cdc700199b80001480084c8cdd81ba8001375000666e00cdc019b823371c00600290402019b823371c00666e00005200248080cdc199b8e0033370000290022404066e0c005200432330010014800088c94ccc14ccdc3800a4000266e012004330030033370000490010a99982999b880014808052002148000cdc70018009919191801000980080111b92001376600266e952000330523752086660a46ea0104cc148dd481f998291ba803d330523750076660a46ea00e52f5c02c66e00cdc199b8100300148010004dd6982880098248048b1bad304f0013047003163370e900118249baa304d001304d002304b00130430053370e900118229baa304900130490023047001303f003304500130450023043001303b011304100130410023756607e002607e002606c0022c6078002646600200202444a666076002297ae013232533303a3375e6606c6070004900000509981f00119802002000899802002000981f801181e8009bae303a0013032001163302f303100348000dd5981b800981b801181a800981680099299981799b8748000c0b80044c8c8cc0b4c0bc00520023035001302d00116323300100100d22533303300114c0103d87a80001323253330323375e6605c60600049000009099ba548000cc0d80092f5c0266008008002606e004606a002646600200200c44a666064002297adef6c6013232323253330333371e911000021003133037337606ea4008dd3000998030030019bab3034003375c6064004606c0046068002606200260620026060002605e0046eacc0b4004c0b4004c0b0008dd61815000981500098148011bac3027001301f0053025001301d0011630230013023002302100130190123758603e002603e002603c0046eb4c070004c070008dd6980d000980d0011bad30180013018002375a602c002602c0046eb8c050004c050008dd6980900098050030a4c2c6eb800cc94ccc02ccdc3a4000002264646464646464646464646464646464a66603c60420042930b1bac301f001301f002301d001301d002375a603600260360046eb4c064004c064008dd6980b800980b8011bad30150013015002375c602600260260046eb4c044004c02401458c024010c034c018004cc0040052000222233330073370e0020060184666600a00a66e000112002300e001002002230053754002460066ea80055cd2ab9d5573caae7d5d02ba157449812bd8799fd8799f5820580c37415cf5b98da27f845ed853f2e4fda0034c1441c99eb3a7f333483ce99dff02ff0001"
const TUNA_VALIDATOR_HASH_MAINNET = "279f842c33eed9054b9e3c70cd6a3b32298259c24b78b895cb41d91a"
const TUNA_VALIDATOR_HASH_PREVIEW = "502fbfbdafc7ddada9c335bd1440781e5445d08bada77dc2032866a6"
const TUNA_VALIDATOR_ADDRESS_MAINNET = "addr1wynelppvx0hdjp2tnc78pnt28veznqjecf9h3wy4edqajxsg7hwsc"
const TUNA_VALIDATOR_ADDRESS_PREVIEW = "addr_test1wpgzl0aa4lramtdfcv6m69zq0q09g3ws3wk6wlwzqv5xdfsdcf2qa"

async function handler(request: Request): Promise<Response> {
  const url = new URL(request.url);
  
  if (url.pathname === "/submit") {
    console.log("POST /submit")
    return handleSubmit(request);
  } else {
    return new Response("Not Found", { status: 404 });
  }
}

interface DenoSubmission {
  nonce: string;
  sha: string;
  current_block: any;
  new_zeroes: number; 
  new_difficulty: number;
  miner_payments: Record<string, number>;  // <address, amount>
}


async function handleSubmit(request: Request): Promise<Response> {
  const answer = await request.json()
  if (!answer.nonce || !answer.current_block || !answer.new_zeroes || !answer.new_difficulty || !answer.miner_payments) {
    return new Response(JSON.stringify({
      message: "sent a bad submission"
    }), { status: 400 })
  }

  return handleSubmitRetrying(answer)
}

const OwnerDataSchema = Data.Object({
  owner_vkh: Data.Bytes()
})
type OwnerData = Data.Static<typeof OwnerDataSchema>
const OwnerData = OwnerDataSchema as unknown as OwnerData

class OwnerDatumCache {
  storeByUtxo: Record<string, OwnerData> = {} // <utxoRef, datum>
  
  async getOwnerDatumByUtxo(utxo: UTxO): Promise<OwnerData | null> {
    const ref = `${utxo.txHash}#${utxo.outputIndex}`
    if (this.storeByUtxo[ref]) {
      return this.storeByUtxo[ref]
    }

    try {
      const datum = await lucid.datumOf<OwnerData>(utxo, OwnerDataSchema)
      if (!datum) {
        return null
      }
      this.storeByUtxo[ref] = datum
      return datum
    } catch (_) {
      return null
    }
  }

  async hydrateCache(): Promise<void> {
    const poolContractUtxos = await lucid.utxosAt(POOL_CONTRACT_ADDRESS);
    
    const ownerDataForUtxos = await Promise.all(poolContractUtxos.map(async (utxo) => {
      const ref = `${utxo.txHash}#${utxo.outputIndex}`
      const ownerDataForUtxo = await ownerDatumCache.getOwnerDatumByUtxo(utxo);
      return { ref, ownerData: ownerDataForUtxo }
    }))

    for (const { ref, ownerData } of ownerDataForUtxos) {
      if (ownerData) {
        this.storeByUtxo[ref] = ownerData;
      }
    }
  }

}

const ownerDatumCache = new OwnerDatumCache()
const CACHE_HYDRATION_INTERVAL = 5000
await ownerDatumCache.hydrateCache()
console.log("Owner Datum cache was hydrated.")

setInterval(() => {
  // this costs nothing if there aren't new utxos at the pool contract, so it can be relatively frequent
  ownerDatumCache.hydrateCache()
}, CACHE_HYDRATION_INTERVAL)

async function handleSubmitRetrying(answer: DenoSubmission, retries = 0): Promise<Response> {
  const validatorHash = network === "Mainnet" ? TUNA_VALIDATOR_HASH_MAINNET : TUNA_VALIDATOR_HASH_PREVIEW
  const tunaValidatorAddress = network === "Mainnet" ? TUNA_VALIDATOR_ADDRESS_MAINNET : TUNA_VALIDATOR_ADDRESS_PREVIEW
  const tunaValidatorCode = network === "Mainnet" ? TUNA_VALIDATOR_CODE_MAINNET : TUNA_VALIDATOR_CODE_PREVIEW

  const validatorUTXOs = await lucid.utxosAt(tunaValidatorAddress);
  const validatorOutRef = validatorUTXOs.find(
    (u) => u.assets[validatorHash + fromText("lord tuna")],
  )

  if (!validatorOutRef) {
    throw Error("No UTXO on the validator address. So either the validator address is wrong, or your infrastructure is mis-reporting the chain state.")
  }

  const realTimeNow = Number((Date.now() / 1000).toFixed(0)) * 1000 - 60000;
  const new_diff = getDifficulty(fromHex(answer.sha))
  const interlink = calculateInterlink(answer.sha, new_diff, {
    leadingZeros: BigInt(answer.current_block.leading_zeroes),
    difficulty_number: BigInt(answer.current_block.difficulty_number),
  }, answer.current_block.interlink.map(toHex) as string[]);

  let epoch_time = BigInt(answer.current_block.epoch_time) + BigInt(90000 + realTimeNow) - BigInt(answer.current_block.current_time)

  let leading_zeroes = BigInt(answer.current_block.leading_zeroes)
  let difficulty_number = BigInt(answer.current_block.difficulty_number)

  if (answer.current_block.block_number % 2016 === 0 && answer.current_block.block_number > 0) {
    const adjustment = getDifficultyAdjustement(epoch_time as unknown as bigint, 1_209_600_000n);
    epoch_time = 0n;


    const new_difficulty = calculateDifficultyNumber(
      {
        leadingZeros: answer.current_block.leading_zeroes as bigint,
        difficulty_number: answer.current_block.difficulty_number as bigint,
      },
      adjustment.numerator,
      adjustment.denominator,
    );

    difficulty_number = new_difficulty.difficulty_number;
    leading_zeroes = new_difficulty.leadingZeros;
  }

  const postDatum = new Constr(0, [
    BigInt(answer.current_block.block_number + 1),
    answer.sha,
    leading_zeroes,
    difficulty_number,
    epoch_time,
    BigInt(90000 + realTimeNow),
    BigInt(0),
    interlink,
  ]);
  const outDat = Data.to(postDatum);
  const tunaAssetName = validatorHash + fromText("TUNA")
  const mintTuna = { [tunaAssetName]: 5000000000n }
  const tunaMasterToken = { [validatorHash + fromText("lord tuna")]: 1n }
  const poolMasterToken = `${POOL_SCRIPT_HASH}${fromText("POOL")}`

  const poolContractUtxos = await lucid.utxosAt(POOL_CONTRACT_ADDRESS)
  // now we need to filter the pool contract utxos by the users who actually need to get paid
  // i.e. do not spend someone else's account 

  const paidAddressToUtxoMap: Record<string, UTxO> = {} // these utxos are "miner accounts" on the contract address
  const utxoRefToPaidAddressMap: Record<string, string> = {}
  for (const [address, _] of Object.entries(answer.miner_payments)) {
    const vkh = lucid.utils.paymentCredentialOf(address)!.hash
    for (const utxo of poolContractUtxos) {
      const ownerDataForUtxo = await ownerDatumCache.getOwnerDatumByUtxo(utxo)
      if (ownerDataForUtxo && ownerDataForUtxo.owner_vkh == vkh) {
        paidAddressToUtxoMap[address] = utxo
        utxoRefToPaidAddressMap[`${utxo.txHash}#${utxo.outputIndex}`] = address
      }
    }
  }


  // only spend utxos representing the accounts getting paid
  const filteredPoolContractUtxos = poolContractUtxos
    .filter(potentialInput => {
      return utxoRefToPaidAddressMap[`${potentialInput.txHash}#${potentialInput.outputIndex}`]
    })

  const poolfundingUtxos = (await lucid.wallet.getUtxos()).filter(utxo => {
    return (
      Object.keys(utxo.assets).length == 1 && utxo.assets["lovelace"]
    ) || utxo.assets[poolMasterToken]
  })

  try {
    const temp_tx = lucid.newTx()
      .collectFrom(
        poolfundingUtxos  // spend the pool's ada and the master token as needed, but never the tuna
      )
      .collectFrom(
        [validatorOutRef],
        Data.to(new Constr(1, [answer.nonce])),
      )
      .collectFrom(
        filteredPoolContractUtxos,
        Data.to(new Constr(1, [new Constr(0, [])]))
      )
      .payToAddressWithData(
        tunaValidatorAddress,
        { inline: outDat },
        tunaMasterToken
      )
      .payToAddress(
        poolRewardAddress,
        { [poolMasterToken]: 1n }
      )
      .mintAssets(mintTuna, Data.to(new Constr(0, [])))
      .attachSpendingValidator({  // TODO: Read from chain instead of having in here, we can publish to preview ourself
        type: "PlutusV2",
        script: tunaValidatorCode
      })
      .readFrom(poolScriptRef)
      .validTo(realTimeNow + 180000)
      .validFrom(realTimeNow)
    
    for (const [address, amount] of Object.entries(answer.miner_payments)) {
      const existingUtxo = paidAddressToUtxoMap[address]
      if (existingUtxo) {
        // this miner has been paid before
        const existingDatum = (await ownerDatumCache.getOwnerDatumByUtxo(existingUtxo))! // contract address is (hopefully) guaranteed to always have valid datums
        const oldTuna = existingUtxo.assets[tunaAssetName]

        // reuse the existing owner datum, and pay them their account balance PLUS what they earned this block
        temp_tx.payToAddressWithData(POOL_CONTRACT_ADDRESS, { inline: Data.to(existingDatum, OwnerData) }, {
          ...existingUtxo.assets,
          [tunaAssetName]: oldTuna + BigInt(amount)
        })
      } else {
        // this miner has never been paid
        // create a new owner datum, and pay them exactly what they earned in this block
        const owner_vkh = lucid.utils.paymentCredentialOf(address)!.hash
        temp_tx.payToAddressWithData(POOL_CONTRACT_ADDRESS, { inline: Data.to({ owner_vkh }, OwnerData) }, {
          [tunaAssetName]: BigInt(amount)
        })
      }
      
    }

    const tx = await temp_tx.complete({ coinSelection: false })
    const signed = await tx.sign().complete()

    const tx_hash = await Promise.race([
      signed.submit(),
      delay(2000)
    ])

    if (tx_hash) {
      console.log(`Successful submission with tx hash ${tx_hash}`)
      return new Response(JSON.stringify({
        message: `Successful submission with hash ${answer.sha}`,
        tx_hash
      }), { status: 200 })
    } else {
      if (retries < 1) {
        console.log(`Submission timed out. Retry ${retries}.`)
        return handleSubmitRetrying(answer, retries + 1)
      } else {
        console.log(`Gave up on submitting sha ${answer.sha}`)
      }
    }

  } catch (e) {
    console.log(`Failed submission!`)
    console.debug(e)
  }

  return new Response(JSON.stringify({
    message: `Could not submit hash ${answer.sha}`,
  }), { status: 500 })
}

console.log("Submission server listening on 22123")
for await (const conn of Deno.listen({ port: 22123 })) {
  (async () => {
    for await (const { request, respondWith } of Deno.serveHttp(conn)) {

      respondWith(handler(request))
        .catch(e => {
          console.error("Received terminal connection state during processing request.")
          console.debug(e)
        })
    }
  })()
}


export function calculateInterlink(
  currentHash: string,
  a: { leadingZeros: bigint; difficulty_number: bigint },
  b: { leadingZeros: bigint; difficulty_number: bigint },
  currentInterlink: string[],
): string[] {
  let b_half = halfDifficultyNumber(b);

  const interlink: string[] = currentInterlink;

  let currentIndex = 0;

  while (
    b_half.leadingZeros < a.leadingZeros ||
    b_half.leadingZeros == a.leadingZeros &&
    b_half.difficulty_number > a.difficulty_number
  ) {
    if (currentIndex < interlink.length) {
      interlink[currentIndex] = currentHash;
    } else {
      interlink.push(currentHash);
    }

    b_half = halfDifficultyNumber(b_half);
    currentIndex += 1;
  }

  return interlink;
}

export function halfDifficultyNumber(
  a: { leadingZeros: bigint; difficulty_number: bigint },
): { leadingZeros: bigint; difficulty_number: bigint } {
  const new_a = a.difficulty_number / 2n;
  if (new_a < 4096n) {
    return {
      leadingZeros: a.leadingZeros + 1n,
      difficulty_number: new_a * 16n,
    };
  } else {
    return {
      leadingZeros: a.leadingZeros,
      difficulty_number: new_a,
    };
  }
}

export function getDifficulty(
  hash: Uint8Array,
): { leadingZeros: bigint; difficulty_number: bigint } {
  let leadingZeros = 0;
  let difficulty_number = 0;
  for (const [indx, chr] of hash.entries()) {
    if (chr !== 0) {
      if ((chr & 0x0F) === chr) {
        leadingZeros += 1;
        difficulty_number += chr * 4096;
        difficulty_number += hash[indx + 1] * 16;
        difficulty_number += Math.floor(hash[indx + 2] / 16);
        return {
          leadingZeros: BigInt(leadingZeros),
          difficulty_number: BigInt(difficulty_number),
        };
      } else {
        difficulty_number += chr * 256;
        difficulty_number += hash[indx + 1];
        return {
          leadingZeros: BigInt(leadingZeros),
          difficulty_number: BigInt(difficulty_number),
        };
      }
    } else {
      leadingZeros += 2;
    }
  }
  return { leadingZeros: 32n, difficulty_number: 0n };
}
export function getDifficultyAdjustement(
  total_epoch_time: bigint,
  epoch_target: bigint,
): { numerator: bigint; denominator: bigint } {
  if (
    epoch_target / total_epoch_time >= 4 && epoch_target % total_epoch_time > 0
  ) {
    return { numerator: 1n, denominator: 4n };
  } else if (
    total_epoch_time / epoch_target >= 4 && total_epoch_time % epoch_target > 0
  ) {
    return { numerator: 4n, denominator: 1n };
  } else {
    return { numerator: total_epoch_time, denominator: epoch_target };
  }
}

export function calculateDifficultyNumber(
  a: { leadingZeros: bigint; difficulty_number: bigint },
  numerator: bigint,
  denominator: bigint,
): { leadingZeros: bigint; difficulty_number: bigint } {
  const new_padded_difficulty = a.difficulty_number * 16n * numerator /
    denominator;

  const new_difficulty = new_padded_difficulty / 16n;

  if (new_padded_difficulty / 65536n == 0n) {
    if (a.leadingZeros >= 62n) {
      return { difficulty_number: 4096n, leadingZeros: 62n };
    } else {
      return {
        difficulty_number: new_padded_difficulty,
        leadingZeros: a.leadingZeros + 1n,
      };
    }
  } else if (new_difficulty / 65536n > 0n) {
    if (a.leadingZeros <= 2) {
      return { difficulty_number: 65535n, leadingZeros: 2n };
    } else {
      return {
        difficulty_number: new_difficulty / 16n,
        leadingZeros: a.leadingZeros - 1n,
      };
    }
  } else {
    return {
      difficulty_number: new_difficulty,
      leadingZeros: a.leadingZeros,
    };
  }
}
