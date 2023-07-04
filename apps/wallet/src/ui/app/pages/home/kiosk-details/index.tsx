// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useGetKioskContents } from '@mysten/core';
import { formatAddress, type SuiObjectResponse } from '@mysten/sui.js';
import { useSearchParams } from 'react-router-dom';

import { useActiveAddress } from '_app/hooks/useActiveAddress';
import Loading from '_components/loading';
import { useGetNFTMeta } from '_hooks';
import { NftImage } from '_src/ui/app/components/nft-display/NftImage';
import { useResolveVideo } from '_src/ui/app/hooks/useResolveVideo';
import PageTitle from '_src/ui/app/shared/PageTitle';

function NFTDisplay({ object }: { object: SuiObjectResponse }) {
	const objectId = object.data?.objectId;

	const { data: nftMeta, isLoading } = useGetNFTMeta(objectId!);
	const nftName = nftMeta?.name || formatAddress(objectId!);
	const nftImageUrl = nftMeta?.imageUrl || '';
	const video = useResolveVideo(object);
	const playable = false;
	// const fileExtensionType = useFileExtensionType(nftImageUrl);
	return (
		<Loading loading={isLoading}>
			{video && playable ? (
				<video controls className="h-full w-full rounded-md overflow-hidden" src={video} />
			) : (
				<NftImage
					name={nftName}
					src={nftImageUrl}
					title={nftMeta?.description || ''}
					animateHover={true}
					showLabel={false}
					borderRadius="md"
					size="md"
					video={video}
				/>
			)}
		</Loading>
	);
}

function KioskDetailsPage() {
	const [searchParams] = useSearchParams();
	const kioskId = searchParams.get('kioskId');
	const accountAddress = useActiveAddress();
	const { data: kioskData } = useGetKioskContents(accountAddress);

	const kiosk = Object.entries(kioskData?.kiosks || {}).find(([, kiosk]) => {
		return kiosk.get(kioskId!);
	});

	const [type, contents] = kiosk || [];
	const items = contents?.get(kioskId!)?.items ?? [];

	return (
		<div className="flex flex-1 flex-col flex-nowrap gap-5">
			<PageTitle title="Kiosk" back="/nfts" />
			<div className="grid grid-cols-3 gap-3 items-center justify-center">
				{items.map((item) => (
					<NFTDisplay object={item} />
				))}
			</div>
		</div>
	);
}

export default KioskDetailsPage;
