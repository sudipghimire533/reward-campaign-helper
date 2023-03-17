#[subxt::subxt(runtime_metadata_path = "chain-metadata.scale")]
pub mod datahighway {}

use datahighway::runtime_types::pallet_reward_campaign::types as campaign_types;
use sp_core::sr25519::Pair as Sr25519Pair;
use sp_keyring::AccountKeyring;
use std::error::Error as StdError;
use std::path::PathBuf;
use subxt::blocks::ExtrinsicEvents;
use subxt::config::Config;
use subxt::tx::TxPayload;
use subxt::{tx, OnlineClient, PolkadotConfig, SubstrateConfig};
use serde::{Serialize, Deserialize};

type DatahighwayOnlineClient = subxt::client::OnlineClient<DatahighwayConfig>;

#[derive(Clone, Copy, Eq, PartialEq, PartialOrd, Ord)]
pub struct DatahighwayConfig;

type AccountId = subxt::utils::AccountId32;
type BlockNumber = u32;
type Balance = u128;
type Index = u32;
impl Config for DatahighwayConfig {
    type Index = Index;
    type Hash = subxt::ext::sp_core::H256;
    type AccountId = AccountId;
    type Address = subxt::ext::sp_runtime::MultiAddress<AccountId, Index>;
    type Hasher = <SubstrateConfig as Config>::Hasher;
    type Header = subxt::config::substrate::SubstrateHeader<BlockNumber, Self::Hasher>;
    type Signature = subxt::ext::sp_runtime::MultiSignature;
    type ExtrinsicParams = <SubstrateConfig as Config>::ExtrinsicParams;
}

type PairSigner = tx::PairSigner<DatahighwayConfig, Sr25519Pair>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let api = OnlineClient::<PolkadotConfig>::new().await?;

    let caller = PairSigner::new(AccountKeyring::Alice.pair());

    Ok(())
}

async fn start_campaign(
    campaign_id: CampaignId,
    campaign_info: CreateCampaignParams,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .start_new_campaign(campaign_id, campaign_info);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn update_campaign(
    campaign_id: CampaignId,
    new_campaign_info: UpdateCampaignParams,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .update_campaign(campaign_id, new_campaign_info);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn discard_campaign(campaign_id: CampaignId) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().discard_campaign(campaign_id);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn wipe_campaign(campaign_id: CampaignId) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().wipe_campaign(campaign_id);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn add_contributer(
    campaign_id: CampaignId,
    contributer: AccountId,
    amount: Balance,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .add_contributer(campaign_id, contributer, amount);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn remove_contributer(
    campaign_id: CampaignId,
    contributer: AccountId,
) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx()
        .reward()
        .remove_contributer(campaign_id, contributer);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn lock_campaign(campaign_id: CampaignId) -> Result<(), Box<dyn StdError>> {
    let call = datahighway::tx().reward().lock_campaign(campaign_id);
    submit_watch_finalize(call).await?;

    Ok(())
}

async fn submit_watch_finalize<Call>(
    call: Call,
) -> Result<ExtrinsicEvents<DatahighwayConfig>, subxt::Error>
where
    Call: TxPayload,
{
    api()
        .tx()
        .sign_and_submit_then_watch(&call, &SIGNER, Default::default())
        .await?
        .wait_for_finalized_success()
        .await
}

static DATAHIGHWAY_API: DatahighwayOnlineClient = {
    // TODO:
    // use lazy_static! to get the api
    todo!()
};

static SIGNER: PairSigner = {
    // TODO:
    // read the private key from user filesystem and make a signer
    todo!()
};

fn api() -> &'static DatahighwayOnlineClient {
    &DATAHIGHWAY_API
}

type CreateCampaignParams = campaign_types::CreateCampaignParam<AccountId, BlockNumber>;
type UpdateCampaignParams = campaign_types::UpdateCampaignParam<AccountId, BlockNumber>;
type CampaignInfo = campaign_types::CampaignReward<AccountId, BlockNumber>;
type CampaignId = u32;


#[derive(Serialize, Deserialize, Debug)]
struct Contributer {
    pub who: AccountId,
    pub contributed: Balance,
    pub contributing: Balance,
}

impl Contributer {
    pub fn reward_amount(&self) -> Balance {
        // convert contributed amount to reward amount
        // FIXME:
        self.contributed.clone()
    }
}

pub struct Campaign {
    pub campaign_id: CampaignId,
    pub instant_percentage: (u32, u32),
    pub starts_from: BlockNumber,
    pub ends_at: BlockNumber,
    pub hoster: AccountId,
    pub contributers: Vec<Contributer>,
}

impl Campaign {
    pub async fn create(&self) -> Result<(), Box<dyn StdError>> {
        start_campaign(self.campaign_id, CreateCampaignParams {
            hoster: Some(self.hoster.clone()),
            instant_percentage:  {
                let (numenator, denomator) = self.instant_percentage;
                campaign_types::SmallRational { numenator, denomator }
            },
            starts_from: Some(self.starts_from),
            end_target: self.ends_at,
        }).await
    }

    pub async fn populate_contributer(&self) -> Result<(), Box<dyn StdError>> {
        for contributer in self.contributers.iter() {
            add_contributer(self.campaign_id, contributer.account.clone(), contributer.reward_amount()).await?;
        }

        Ok(())
    }
}

pub struct InputFile {
    pub campaign: Campaign,
    pub contributers_file: PathBuf,
}