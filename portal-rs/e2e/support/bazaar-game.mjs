// TEST ONLY: fixture-backed Game producer. Never imported by production Rust.
// Source: Oteryn/Oteryn-Game@3b49a20feec3ba29700bb4b2fa28fa9b539b79a2
// docs/contracts/fixtures/bazaar/decisions.json; exact copies live in the crate.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import https from 'node:https';
import http from 'node:http';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const SOURCE='3b49a20feec3ba29700bb4b2fa28fa9b539b79a2';
const fixtureFile=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'../../crates/bazaar/tests/fixtures/plat3/decisions.json');
const rawFixtures=fs.readFileSync(fixtureFile);
const fixtures=JSON.parse(rawFixtures);
const sha=raw=>crypto.createHash('sha256').update(raw).digest('hex');
for(const vector of fixtures.vectors) if(sha(vector.wire)!==vector.sha256) throw new Error('Pinned Bazaar fixture hash mismatch');
const fixture=name=>JSON.parse(fixtures.vectors.find(v=>v.name===name).wire);
export function uuid7(){const bytes=crypto.randomBytes(16);let time=BigInt(Date.now());for(let i=5;i>=0;i--){bytes[i]=Number(time&255n);time>>=8n;}bytes[6]=(bytes[6]&15)|112;bytes[8]=(bytes[8]&63)|128;const h=bytes.toString('hex');return `${h.slice(0,8)}-${h.slice(8,12)}-${h.slice(12,16)}-${h.slice(16,20)}-${h.slice(20)}`;}
const uuidBytes=id=>Buffer.from(id.replaceAll('-',''),'hex');
const validID=id=>typeof id==='string'&&/^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(id);
const now=()=>Math.floor(Date.now()/1000);
const inc=n=>String(BigInt(n)+1n);
const sleep=ms=>new Promise(resolve=>setTimeout(resolve,ms));
function varint(value){let n=BigInt(value);const bytes=[];while(n>127n){bytes.push(Number(n&127n)|128);n>>=7n;}bytes.push(Number(n));return Buffer.from(bytes);}
const integer=(tag,n)=>Buffer.concat([varint(tag*8),varint(n)]);
const bytes=(tag,b)=>Buffer.concat([varint(tag*8+2),varint(b.length),b]);
const text=(tag,s)=>bytes(tag,Buffer.from(s));
function event(intent,receipt,operation,world,generation){
 const payload=Buffer.concat([bytes(1,crypto.createHash('sha256').update(intent).digest()),bytes(2,crypto.createHash('sha256').update(receipt).digest()),integer(3,generation)]);
 const transaction=Buffer.concat([bytes(1,uuidBytes(uuid7())),integer(2,1),integer(3,1)]);
 return Buffer.concat([integer(1,1),bytes(2,uuidBytes(uuid7())),integer(3,6),integer(4,1),integer(5,2),integer(6,3),text(7,'PLAT3_AUTHORITY_AUDIT_RETENTION_V1'),integer(8,Date.now()),bytes(9,uuidBytes(world)),bytes(17,uuidBytes(operation)),bytes(18,transaction),text(25,'fixture-bazaar-game-v1'),bytes(26,payload),bytes(27,crypto.createHash('sha256').update(payload).digest())]);
}
export function prepareBazaar(env,scratch){
 const directory=path.join(scratch,'bazaar-game');fs.mkdirSync(directory,{recursive:true,mode:0o700});
 const exec=args=>{const result=spawnSync('openssl',args,{stdio:'ignore'});if(result.status!==0)throw new Error('Local Bazaar certificate generation failed');};
 const identities={};
 for(const name of ['bazaar','character']){
  const key=path.join(directory,`${name}-key.pem`),csr=path.join(directory,`${name}.csr`),cert=path.join(directory,`${name}-cert.pem`),ext=path.join(directory,`${name}.ext`);
  exec(['req','-newkey','rsa:2048','-nodes','-keyout',key,'-out',csr,'-subj',`/CN=local-${name}-producer`]);
  fs.writeFileSync(ext,'basicConstraints=critical,CA:FALSE\nkeyUsage=critical,digitalSignature\nextendedKeyUsage=clientAuth\n');
  exec(['x509','-req','-in',csr,'-CA',`${scratch}/ca.pem`,'-CAkey',`${scratch}/ca-key.pem`,'-CAcreateserial','-out',cert,'-days','2','-extfile',ext]);
  const certificate=fs.readFileSync(cert);const publicKey=new crypto.X509Certificate(certificate).publicKey.export({type:'spki',format:'der'});
  identities[name]={key:fs.readFileSync(key),cert:certificate,spki:sha(publicKey)};
 }
 const signing=crypto.generateKeyPairSync('ed25519');
 fs.writeFileSync(path.join(directory,'signer-private.pem'),signing.privateKey.export({type:'pkcs8',format:'pem'}),{mode:0o600});
 const keysFile=path.join(directory,'snapshot-keys.json');
 fs.writeFileSync(keysFile,JSON.stringify([{kid:'local-bazaar-snapshot-1',public_key_hex:signing.publicKey.export({type:'spki',format:'der'}).subarray(-32).toString('hex')}]),{mode:0o600});
 const fenceFile=path.join(directory,'fence.json');
 fs.writeFileSync(fenceFile,JSON.stringify({projection_epoch:3,recovery_generation:3,incarnation:uuid7(),character_history_complete_since:1}),{mode:0o600});
 const world='0192a000-0000-7000-8000-000000000012',escrow='0192a000-0000-7000-8000-000000000040';
 Object.assign(env,{OTERYN_BAZAAR_ENABLED:'true',OTERYN_BAZAAR_FENCE_FILE:fenceFile,OTERYN_BAZAAR_SNAPSHOT_KEYS_FILE:keysFile,OTERYN_BAZAAR_INTERNAL_BIND:'127.0.0.1:18992',OTERYN_BAZAAR_INTERNAL_BASE_URL:'https://localhost:18992',OTERYN_BAZAAR_TLS_CERT_FILE:`${scratch}/cert.pem`,OTERYN_BAZAAR_TLS_KEY_FILE:`${scratch}/key.pem`,OTERYN_BAZAAR_CLIENT_CA_FILE:`${scratch}/ca.pem`,OTERYN_BAZAAR_CLIENT_SPKI_SHA256:identities.bazaar.spki,OTERYN_BAZAAR_RECEIPT_CLIENT_SPKI_SHA256:identities.character.spki,OTERYN_BAZAAR_RECEIPT_AUTHORITY:'oteryn:character-authority:primary',OTERYN_BAZAAR_SOURCE_AUTHORITY:'oteryn:game:primary',OTERYN_BAZAAR_ESCROW_ACCOUNT_ID:escrow,OTERYN_BAZAAR_WORLD_IDS:world});
 return {directory,identities,signing,ca:fs.readFileSync(`${scratch}/ca.pem`),env,world,escrow,fence:fs.readFileSync(fenceFile,'utf8')};
}
export async function startBazaarProducer(config){
 const {env,directory,world,escrow}=config;
 const journal=path.join(directory,'game-journal.json');
 let state=fs.existsSync(journal)?JSON.parse(fs.readFileSync(journal,'utf8')):{assets:{},characters:{},decisions:{},statuses:{},accountRevisions:{},outbox:[],errors:[]};
 state.banks??={};state.holds??={};state.sales??={};
 let closing=false,lastWatermark=0,policyAcknowledged=false;
 function persist(){const temporary=`${journal}.tmp`;fs.writeFileSync(temporary,JSON.stringify(state),{mode:0o600});fs.renameSync(temporary,journal);}
 function fence(){if(fs.readFileSync(env.OTERYN_BAZAAR_FENCE_FILE,'utf8')!==config.fence)throw new Error('Recovery fence changed');}
 const agents=Object.fromEntries(Object.entries(config.identities).map(([name,identity])=>[name,new https.Agent({key:identity.key,cert:identity.cert,ca:config.ca,keepAlive:true,maxSockets:1,minVersion:'TLSv1.3',maxVersion:'TLSv1.3',ALPNProtocols:['http/1.1']})]));
 async function post(peer,pathname,body,binary=false){
  const target=new URL(pathname,env.OTERYN_BAZAAR_INTERNAL_BASE_URL);
  return await new Promise((resolve,reject)=>{
   const request=https.request(target,{method:'POST',agent:agents[peer],headers:{'content-type':binary?'application/x-protobuf':'application/json','content-length':body.length}},response=>{const chunks=[];let size=0;response.on('data',chunk=>{size+=chunk.length;if(size>270336){request.destroy(new Error('Response bound'));return;}chunks.push(chunk);});response.on('end',()=>resolve({status:response.statusCode,body:Buffer.concat(chunks).toString()}));});
   request.setTimeout(pathname.includes('/commands/')?3000:5000,()=>request.destroy(new Error('Exchange timeout')));request.on('error',reject);request.end(body);
  });
 }
 const head=(operation,version=2)=>({contract_version:version,operation,source_authority:version===1?env.OTERYN_BAZAAR_RECEIPT_AUTHORITY:env.OTERYN_BAZAAR_SOURCE_AUTHORITY});
 function status(i,asset,current,operation){
  const previous=state.statuses[i.listing_id];
  const value={...head('PublishBazaarListingStatusV1'),listing_id:i.listing_id,seller_account_id:i.seller_account_id,world_id:asset.world,projection_epoch:'3',projection_revision:previous?inc(previous.projection_revision):'1',source_observed_at:String(now()),asset_kind:asset.kind==='character'?'CHARACTER':'ITEM',asset_id:asset.id,state:current,custody_id:asset.kind==='character'?asset.lock:asset.escrow,custody_revision:asset.custodyRevision,snapshot_hash:asset.kind==='character'?asset.snapshotHash:null,last_operation_id:operation,reason_code:null};
  state.statuses[i.listing_id]=value;return JSON.stringify(value);
 }
 function item(asset){const value=fixture('escrow_receipt').result_data.item;value.item_instance_id=asset.id;value.world_id=asset.world;value.state_revision=asset.revision;value.quantity=asset.quantity;return value;}
 function lockSnapshot(i,asset){
  const signed=fixture('locked_character_snapshot'),s=signed.snapshot;
  Object.assign(s,{source_authority:env.OTERYN_BAZAAR_SOURCE_AUTHORITY,listing_id:i.listing_id,seller_account_id:i.seller_account_id,character_id:asset.id,world_id:asset.world,lock_id:asset.lock,lock_revision:asset.custodyRevision,character_revision:asset.revision,eligibility_revision:'4',projection_epoch:'3',captured_at:String(now()),name:asset.name});
  signed.snapshot_hash=sha(JSON.stringify(s));signed.kid='local-bazaar-snapshot-1';signed.signature=crypto.sign(null,Buffer.from(`oteryn-v2-bazaar-character-snapshot-v1.${Buffer.from(JSON.stringify(s)).toString('base64url')}`),config.signing.privateKey).toString('base64url');asset.snapshotHash=signed.snapshot_hash;return signed;
 }
 function fail(code){const error=new Error(code);error.resultCode=code;throw error;}
 function requireFact(condition,code='CHAR_CMD_BAZAAR_STALE_REVISION'){if(!condition)fail(code);}
 function bank(account){return state.banks[account]??={balance:50000,held:0,incoming:0};}
 function holdFor(i){return state.holds[`${i.listing_id}:${i.buyer_account_id}`];}
 function funding(i,asset){
  requireFact(validID(i.funding?.reservation_id)&&BigInt(i.funding.gross_amount)>0n,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
  if(i.funding.currency==='COINS')return;
  requireFact(i.funding.currency==='GOLD','CHAR_CMD_BAZAAR_HOLD_CONFLICT');
  const h=Object.values(state.holds).find(h=>h.id===i.funding.reservation_id),gross=Number(i.funding.gross_amount),fee=Math.ceil(gross*250/10000),net=gross-fee;
  requireFact(h&&h.listing===i.listing_id&&(!i.buyer_account_id||h.buyer===i.buyer_account_id)&&h.revision===i.funding.hold_revision&&h.amount>=gross&&!h.sale,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
  const seller=bank(i.seller_account_id);requireFact(seller.balance+seller.incoming+net<=999999999999,'CHAR_CMD_BAZAAR_SELLER_BANK_CAP');
  h.sale=i.sale_id;seller.incoming+=net;state.sales[i.sale_id]={buyer:h.buyer,seller:i.seller_account_id,listing:i.listing_id,hold:h.id,gross,fee,net,asset:asset.id,payout:false};
 }
 function bankCommand(i){
  const buyer=bank(i.buyer_account_id),key=`${i.listing_id}:${i.buyer_account_id}`;
  let h=state.holds[key];
  if(i.command==='HoldBazaarGold'){
   requireFact(state.characters[i.buyer_character_id]?.owner===i.buyer_account_id,'CHAR_CMD_OWNER_MISMATCH');
   requireFact((h?.revision||'0')===i.expected_hold_revision&&!h?.sale,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
   const amount=Number(i.amount),old=h?.amount||0;
   requireFact(buyer.balance-buyer.held+old>=amount,'CHAR_CMD_BAZAAR_INSUFFICIENT_GOLD');
   buyer.held+=amount-old;h={id:h?.id||uuid7(),revision:inc(h?.revision||'0'),amount,buyer:i.buyer_account_id,listing:i.listing_id};state.holds[key]=h;
   return {hold_id:h.id,hold_revision:h.revision,buyer_account_id:i.buyer_account_id,offer_id:i.offer_id,hold_purpose:i.hold_purpose,held_amount:String(amount)};
  }
  requireFact(h&&h.id===i.hold_id&&h.revision===i.expected_hold_revision,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
  if(i.command==='ReleaseBazaarGold'){
   requireFact(!h.sale||state.sales[h.sale]?.payout,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
   const amount=Number(i.amount);requireFact(amount>0&&h.amount>=amount,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
   h.amount-=amount;buyer.held-=amount;h.revision=inc(h.revision);
   return {hold_id:h.id,hold_revision:h.revision,released_amount:String(amount),remaining_held:String(h.amount),hold_state:h.amount?'ACTIVE':'RELEASED'};
  }
  const sale=state.sales[i.sale_id];
  const d=Object.values(state.decisions).find(d=>d.operation===i.delivery_operation_id&&d.outcome==='COMMITTED');
  requireFact(sale&&!sale.payout&&sale.hold===h.id&&sale.buyer===i.buyer_account_id&&sale.seller===i.seller_account_id&&sale.listing===i.listing_id&&d&&sha(d.receipt)===i.delivery_receipt_digest,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
  const asset=state.assets[sale.asset];requireFact(asset.owner===sale.buyer,'CHAR_CMD_OWNER_MISMATCH');
  const gross=Number(i.gross_amount),fee=Math.ceil(gross*250/10000),net=gross-fee;
  requireFact(gross===sale.gross&&fee===Number(i.fee_amount)&&net===Number(i.seller_amount)&&net>0&&h.amount>=gross&&buyer.balance>=gross,'CHAR_CMD_BAZAAR_HOLD_CONFLICT');
  const seller=bank(sale.seller);requireFact(seller.balance+seller.incoming<=999999999999,'CHAR_CMD_BAZAAR_SELLER_BANK_CAP');
  buyer.balance-=gross;buyer.held-=gross;seller.balance+=net;seller.incoming-=net;h.amount-=gross;h.revision=inc(h.revision);sale.payout=true;
  return {hold_id:h.id,hold_revision:h.revision,sale_id:i.sale_id,gross_amount:i.gross_amount,fee_amount:i.fee_amount,seller_amount:i.seller_amount,remaining_held:String(h.amount),buyer_ledger_entry_id:uuid7(),seller_ledger_entry_id:uuid7(),burn_entry_id:uuid7(),delivery_operation_id:i.delivery_operation_id,delivery_receipt_digest:i.delivery_receipt_digest};
 }
 function decide(raw,version){
  fence();const i=JSON.parse(raw);requireFact(JSON.stringify(i)===raw,'CHAR_CMD_INTENT_INVALID');
  const key=`${i.issuer_authority}:${i.operation_id}`;
  if(state.decisions[key]){if(state.decisions[key].intentHash!==sha(raw))throw new Error('Conflicting command bytes');return;}
  const before=structuredClone(state);
  let facts=null,receipt,publication=null,resultCode='CHAR_CMD_OK';
  try{
   requireFact(validID(i.operation_id),'CHAR_CMD_INTENT_INVALID');
   requireFact(BigInt(i.issued_at_source)<=BigInt(now()+5)&&BigInt(i.expires_at_source)>BigInt(now()),'CHAR_CMD_INTENT_EXPIRED');
   requireFact(i.contract_version===version,'CHAR_CMD_UNSUPPORTED');
   const target=i.item_instance_id||i.character_id;
   const asset=Object.values(state.assets).find(a=>(target&&a.id===target)||(i.escrow_id&&a.escrow===i.escrow_id)||(i.lock_id&&a.lock===i.lock_id));
   const bankOperation=version===2&&['HoldBazaarGold','ReleaseBazaarGold','PayoutBazaarGold'].includes(i.command);
   requireFact(bankOperation||asset,'CHAR_CMD_CHARACTER_NOT_FOUND');
   requireFact(bankOperation?i.world_id===world:asset.world===(i.world_id||asset.world),'CHAR_CMD_INTENT_INVALID');
   if(version===2){
    requireFact(i.issuer_authority==='OTERYN_PLATFORM_BAZAAR_AUTHORITY'&&i.audience==='OTERYN_GAME_BAZAAR_AUTHORITY','CHAR_CMD_INTENT_INVALID');
    if(bankOperation){facts=bankCommand(i);}
    else if(i.command==='EscrowBazaarItem'){
     requireFact(asset.kind==='item'&&asset.owner===i.seller_account_id&&asset.character===i.seller_character_id,'CHAR_CMD_OWNER_MISMATCH');
     requireFact(asset.playable&&asset.transferable,'CHAR_CMD_BAZAAR_ITEM_NOT_TRANSFERABLE');
     requireFact(asset.revision===i.expected_state_revision&&i.quantity===asset.quantity&&i.transferability_revision==='4');
     asset.playable=false;asset.escrow=uuid7();asset.custodyRevision='1';asset.revision=inc(asset.revision);asset.listing=i.listing_id;
     facts={escrow_id:asset.escrow,escrow_revision:asset.custodyRevision,seller_character_id:asset.character,item:item(asset)};publication=status(i,asset,'ESCROWED',i.operation_id);
    }else if(i.command==='DeliverBazaarItem'||i.command==='ReturnBazaarItem'){
     requireFact(asset.kind==='item'&&asset.listing===i.listing_id&&asset.custodyRevision===i.expected_escrow_revision&&!asset.playable&&!asset.consumed);
     const deposit=Object.values(state.decisions).find(d=>d.command==='EscrowBazaarItem'&&d.listing===i.listing_id&&d.outcome==='COMMITTED');
     requireFact(deposit&&sha(deposit.receipt)===i.escrow_receipt_digest,'CHAR_CMD_INTENT_INVALID');
     const account=i.command==='DeliverBazaarItem'?i.buyer_account_id:i.seller_account_id;
     const character=i.command==='DeliverBazaarItem'?i.buyer_character_id:i.seller_character_id;
     requireFact(state.characters[character]?.owner===account&&state.characters[character]?.world===asset.world,'CHAR_CMD_BAZAAR_DESTINATION_UNAVAILABLE');
     if(i.command==='DeliverBazaarItem')funding(i,asset);
     asset.owner=account;asset.character=character;asset.inbox=account;asset.consumed=true;asset.custodyRevision=inc(asset.custodyRevision);asset.revision=inc(asset.revision);
     facts={escrow_id:asset.escrow,escrow_revision:asset.custodyRevision,destination_account_id:account,destination_character_id:character,delivery_id:uuid7(),item:item(asset)};publication=status(i,asset,i.command==='DeliverBazaarItem'?'DELIVERED':'RETURNED',i.operation_id);
    }else if(i.command==='LockBazaarCharacter'){
     requireFact(asset.kind==='character'&&asset.owner===i.seller_account_id,'CHAR_CMD_OWNER_MISMATCH');
     requireFact(asset.offline===true,'CHAR_CMD_CHARACTER_IN_SESSION');requireFact(asset.eligible===true&&asset.obligationsResolved===true&&!asset.locked,'CHAR_CMD_BAZAAR_OBLIGATION');
     requireFact(asset.revision===i.expected_character_revision&&i.eligibility_revision==='4');
     asset.locked=true;asset.lock=uuid7();asset.custodyRevision='1';asset.listing=i.listing_id;asset.originalSeller=i.seller_account_id;
     facts={character_id:asset.id,lock_id:asset.lock,lock_revision:asset.custodyRevision,eligibility_revision:'4',signed_snapshot:lockSnapshot(i,asset)};publication=status(i,asset,'locked',i.operation_id);
    }else if(i.command==='BindBazaarCharacterTransfer'){
     requireFact(asset.kind==='character'&&asset.locked&&asset.listing===i.listing_id&&asset.lock===i.lock_id&&asset.custodyRevision===i.expected_lock_revision&&!asset.binding);
     if(i.purpose==='BAZAAR_SETTLEMENT')funding(i,asset);
     asset.binding={operation:i.transfer_operation_id,digest:i.transfer_binding_digest,purpose:i.purpose,sale:i.sale_id};asset.custodyRevision=inc(asset.custodyRevision);
     facts={character_id:asset.id,lock_id:asset.lock,lock_revision:asset.custodyRevision,transfer_operation_id:i.transfer_operation_id,transfer_binding_digest:i.transfer_binding_digest};publication=status(i,asset,'locked',i.operation_id);
    }else if(i.command==='UnlockBazaarCharacter'){
     requireFact(asset.kind==='character'&&asset.locked&&!asset.binding&&asset.lock===i.lock_id&&asset.custodyRevision===i.expected_lock_revision);
     requireFact(asset.owner!==escrow,'CHAR_CMD_OWNER_MISMATCH');asset.locked=false;asset.custodyRevision=inc(asset.custodyRevision);
     facts={character_id:asset.id,lock_id:asset.lock,lock_revision:asset.custodyRevision,current_account_id:asset.owner};publication=status(i,asset,'UNLOCKED',i.operation_id);
    }else fail('CHAR_CMD_UNSUPPORTED');
    receipt={contract_version:2,operation:'PublishCharacterCommandReceiptV1',source_authority:env.OTERYN_BAZAAR_RECEIPT_AUTHORITY,command:i.command,operation_id:i.operation_id,outcome:'COMMITTED',result_code:'CHAR_CMD_OK',result_data:facts,recovery_generation:'3',decided_at:String(now())};
   }else{
    requireFact(i.command==='TransferCharacterOwnership'&&i.issuer_authority==='OTERYN_PLATFORM_CHARACTER_AUTHORITY'&&i.audience==='OTERYN_GAME_CHARACTER_AUTHORITY','CHAR_CMD_UNSUPPORTED');
    requireFact(asset.kind==='character'&&asset.locked&&asset.binding?.operation===i.operation_id&&asset.binding?.digest===sha(raw)&&asset.binding?.purpose===i.purpose,'CHAR_CMD_CHARACTER_NOT_TRANSFERABLE');
    requireFact(asset.owner===i.expected_from_account_id,'CHAR_CMD_OWNER_MISMATCH');
    if(i.purpose==='BAZAAR_LISTING_ESCROW')requireFact(i.to_account_id===escrow&&i.expected_from_account_id===asset.originalSeller,'CHAR_CMD_OWNER_MISMATCH');
    else if(i.purpose==='BAZAAR_SETTLEMENT')requireFact(i.expected_from_account_id===escrow&&i.to_account_id!==asset.originalSeller&&state.characters[Object.keys(state.characters).find(c=>state.characters[c].owner===i.to_account_id)],'CHAR_CMD_OWNER_MISMATCH');
    else if(i.purpose==='BAZAAR_ESCROW_RETURN')requireFact(i.expected_from_account_id===escrow&&i.to_account_id===asset.originalSeller,'CHAR_CMD_OWNER_MISMATCH');else fail('CHAR_CMD_INTENT_INVALID');
    if(i.purpose==='BAZAAR_SETTLEMENT'&&state.sales[asset.binding.sale])requireFact(state.sales[asset.binding.sale].buyer===i.to_account_id,'CHAR_CMD_OWNER_MISMATCH');
    const from=asset.owner;asset.owner=i.to_account_id;state.characters[asset.id].owner=asset.owner;asset.revision=inc(asset.revision);asset.custodyRevision=inc(asset.custodyRevision);asset.binding=null;
    state.accountRevisions[from]=inc(state.accountRevisions[from]||'1');state.accountRevisions[asset.owner]=inc(state.accountRevisions[asset.owner]||'1');
    receipt={contract_version:1,operation:'PublishCharacterCommandReceiptV1',source_authority:env.OTERYN_BAZAAR_RECEIPT_AUTHORITY,command:i.command,operation_id:i.operation_id,outcome:'COMMITTED',result_code:'CHAR_CMD_OK',character_id:asset.id,from_account_id:from,to_account_id:asset.owner,character_revision:asset.revision,projection_epoch:'3',from_projection_revision:state.accountRevisions[from],to_projection_revision:state.accountRevisions[asset.owner],recovery_generation:'3',decided_at:String(now())};
    publication=status({listing_id:asset.listing,seller_account_id:asset.originalSeller},asset,i.purpose==='BAZAAR_SETTLEMENT'?'DELIVERED':i.purpose==='BAZAAR_ESCROW_RETURN'?'RETURNED':'locked',i.operation_id);
   }
  }catch(error){
   if(!error.resultCode)throw error;state=before;resultCode=error.resultCode;
   receipt={contract_version:version,operation:'PublishCharacterCommandReceiptV1',source_authority:env.OTERYN_BAZAAR_RECEIPT_AUTHORITY,command:i.command,operation_id:i.operation_id,outcome:'REJECTED',result_code:resultCode,...(version===2?{result_data:null}:{}),recovery_generation:'3',decided_at:String(now())};
  }
  const receiptRaw=JSON.stringify(receipt);
  const record={key,operation:i.operation_id,command:i.command,listing:i.listing_id||Object.values(state.assets).find(a=>a.id===i.character_id)?.listing,intentHash:sha(raw),receipt:receiptRaw,outcome:receipt.outcome};
  state.decisions[key]=record;
  state.outbox.push({key,version,receipt:receiptRaw,status:publication,event:version===2?event(raw,receiptRaw,i.operation_id,i.world_id,3).toString('base64'):null,receiptSent:false,statusSent:!publication,eventSent:version!==2,blocked:false});
  // Effect + immutable receipt + exact payload + outbox survive the same atomic rename.
  persist();
 }
 async function flush(){
  for(const entry of state.outbox){
   if(entry.blocked)continue;
   for(const phase of ['receipt','status','event']){
    if(entry[`${phase}Sent`])continue;
    const response=phase==='receipt'?await post('character','/internal/v1/game-auth/character-commands/receipt',Buffer.from(entry.receipt)):
     phase==='status'?await post('bazaar','/internal/v1/bazaar/status',Buffer.from(entry.status)):
      await post('bazaar','/internal/v1/game-events',Buffer.from(entry.event,'base64'),true);
    if(response.status===404||response.status===409){entry.blocked=true;state.errors.push('publication conflict; evidence retained');persist();break;}
    if(response.status!==200)return false;
    entry[`${phase}Sent`]=true;persist();
   }
  }
  return state.outbox.every(entry=>!entry.blocked&&entry.receiptSent&&entry.statusSent&&entry.eventSent);
 }
 async function poll(version){
  const namespace=version===2?'/internal/v1/bazaar/commands':'/internal/v1/game-auth/character-commands';
  const peer=version===2?'bazaar':'character';let revision='0',operation='00000000-0000-0000-0000-000000000000';const started=Date.now();
  for(let page=0;page<8&&Date.now()-started<30000;page++){
   const pending={...head(version===2?'ListPendingBazaarCommandsV1':'ListPendingCharacterCommandsV1',version),after_source_revision:revision,after_operation_id:operation,max_entries:32};
   const response=await post(peer,`${namespace}/pending`,Buffer.from(JSON.stringify(pending)));if(response.status!==200)return;
   const entries=JSON.parse(response.body).pending;
   for(const entry of entries){
    if(version===1&&entry.command!=='TransferCharacterOwnership')continue;
    const request={...head(version===2?'ReadBazaarCommandIntentV1':'ReadCharacterCommandIntentV1',version),issuer_authority:version===2?'OTERYN_PLATFORM_BAZAAR_AUTHORITY':'OTERYN_PLATFORM_CHARACTER_AUTHORITY',command:entry.command,operation_id:entry.operation_id};
    const read=await post(peer,`${namespace}/read`,Buffer.from(JSON.stringify(request)));if(read.status!==200)continue;
    decide(read.body,version);await flush();
   }
   if(entries.length<32)break;revision=entries.at(-1).source_revision;operation=entries.at(-1).operation_id;
  }
 }
 const control=http.createServer(async(req,res)=>{
  try{
   if(req.method==='GET'&&req.url==='/fixtures'){res.writeHead(200,{'content-type':'application/json','x-fixture-source':SOURCE});res.end(rawFixtures);return;}
   if(req.method==='GET'&&req.url.startsWith('/state')){const asset=new URL(req.url,'http://localhost').searchParams.get('asset');res.setHeader('content-type','application/json');res.end(JSON.stringify({source:SOURCE,ready:policyAcknowledged,asset:asset?state.assets[asset]:null,decisions:Object.values(state.decisions).map(d=>({operation:d.operation,command:d.command,outcome:d.outcome})),errors:state.errors,banks:state.banks}));return;}
   let raw='';for await(const chunk of req){raw+=chunk;if(raw.length>8192)throw new Error('Body bound');}
   if(req.method==='POST'&&req.url==='/seed'){
    const input=JSON.parse(raw);for(const key of ['seller','buyer','asset_id','seller_character_id','buyer_character_id'])if(!validID(input[key]))throw new Error('Invalid fixture ID');
    if(state.assets[input.asset_id])throw new Error('Fixture identity reuse');
    const base={id:input.asset_id,world,owner:input.seller,character:input.seller_character_id,kind:input.kind,revision:'7',quantity:1,name:'Fixture Hero',playable:true,transferable:!input.refuse,offline:true,eligible:true,obligationsResolved:true,locked:false};
    state.banks[input.seller]??={balance:0,held:0,incoming:0};bank(input.buyer);
    state.characters[input.seller_character_id]={owner:input.seller,world};state.characters[input.buyer_character_id]={owner:input.buyer,world};state.assets[input.asset_id]=base;persist();res.writeHead(201);res.end('{}');return;
   }
   if(req.method==='POST'&&req.url==='/replay'){const input=JSON.parse(raw);if(!validID(input.listing_id))throw new Error('Missing replay subject');for(const entry of state.outbox){if(state.decisions[entry.key].listing!==input.listing_id)continue;entry.receiptSent=false;entry.statusSent=!entry.status;entry.eventSent=!entry.event;}persist();res.end('{}');return;}
   res.writeHead(404);res.end();
  }catch{res.writeHead(400);res.end('{"error":"invalid local fixture request"}');}
 });
 await new Promise(resolve=>control.listen(18993,'127.0.0.1',resolve));
 const run=(async()=>{while(!closing){try{
  fence();
  if(!policyAcknowledged){const item=fixture('escrow_receipt').result_data.item;const policy={...head('PublishBazaarTransferabilityV1'),world_id:world,projection_epoch:'3',projection_revision:'4',source_observed_at:String(now()),eligibility_revision:'4',definitions:[{definition:item.definition,tradeable:true,max_quantity:1,allowed_attribute_keys:['tier']}]};policyAcknowledged=(await post('bazaar','/internal/v1/bazaar/transferability',Buffer.from(JSON.stringify(policy)))).status===200;}
  await flush();await poll(2);await poll(1);
  if(await flush()&&policyAcknowledged&&now()-lastWatermark>=10){const watermark={...head('PublishBazaarWatermarkV1'),projection_epoch:'3',complete_through:String(now()),observed_at:String(now())};if((await post('bazaar','/internal/v1/bazaar/watermark',Buffer.from(JSON.stringify(watermark)))).status===200)lastWatermark=now();}
 }catch(error){if(!['ECONNREFUSED','ECONNRESET'].includes(error.code)&&!error.message.includes('timeout')&&!closing){state.errors.push('producer exchange unavailable');if(state.errors.length>20)state.errors.shift();persist();}}await sleep(1000);}})();
 return {async close(){closing=true;for(const agent of Object.values(agents))agent.destroy();await new Promise(resolve=>control.close(resolve));await run;}};
}
