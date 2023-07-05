// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { hasDisplayData, useGetKioskContents } from '@mysten/core';
import { formatAddress } from '@mysten/sui.js';
import { useSearchParams, Link } from 'react-router-dom';

import { useActiveAddress } from '_app/hooks/useActiveAddress';
import { LabelValueItem } from '_src/ui/app/components/LabelValueItem';
import { LabelValuesContainer } from '_src/ui/app/components/LabelValuesContainer';
import { ErrorBoundary } from '_src/ui/app/components/error-boundary';
import PageTitle from '_src/ui/app/shared/PageTitle';
import { Collapse } from '_src/ui/app/shared/collapse';
import { NFTDisplayCard } from '_src/ui/app/components/nft-display';

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
		<div className="flex flex-1 flex-col flex-nowrap gap-5 mb-10">
			<PageTitle title="Kiosk" back="/nfts" />
			<div className="grid grid-cols-3 gap-3 items-center justify-center mb-auto">
				{items
					.sort((item) => (hasDisplayData(item) ? -1 : 1))
					.map((item) => (
						<Link
							to={`/nft-details?${new URLSearchParams({
								objectId: item.data?.objectId!,
							}).toString()}`}
							onClick={() => {
								// todo: amplitude
							}}
							key={item.data?.objectId}
							className="no-underline"
						>
							<ErrorBoundary>
								<NFTDisplayCard
									objectId={item.data?.objectId!}
									size="md"
									showLabel
									animateHover
									borderRadius="xl"
								/>
							</ErrorBoundary>
						</Link>
					))}
			</div>
			<Collapse initialIsOpen title="Details">
				<LabelValuesContainer>
					<LabelValueItem label="Number of Items" value={items.length} />
					<LabelValueItem label="Kiosk ID" value={formatAddress(kioskId!)} />
					<LabelValueItem label="Type" value={type} />
				</LabelValuesContainer>
			</Collapse>
		</div>
	);
}

export default KioskDetailsPage;
