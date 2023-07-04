// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { hasDisplayData, useGetKioskContents } from '@mysten/core';
import { type SuiObjectResponse } from '@mysten/sui.js';

import { useActiveAddress } from '../../hooks';
import { Text } from '../../shared/text';
import { NftImage } from './NftImage';

export function Kiosk({ object }: { object?: SuiObjectResponse }) {
	const address = useActiveAddress();
	const { data: kioskData } = useGetKioskContents(address);
	const kioskId =
		typeof object?.data?.content === 'object' && 'fields' in object?.data?.content
			? object?.data?.content?.fields.for || object.data.content.fields.kiosk
			: null;
	const suiKiosk = kioskData?.kiosks.sui.get(kioskId);
	const obKiosk = kioskData?.kiosks.originByte.get(kioskId);
	const items = suiKiosk?.items || obKiosk?.items;
	return (
		<div className="relative h-36 w-36 bg-gray-40 ease-ease-out-cubic origin-bottom duration-250 hover:bg-transparent">
			{items
				?.filter(hasDisplayData)
				.slice(0, 3)
				.map((image, idx) => (
					<div
						className={`transition-all ease-ease-out-cubic shadow-md rounded-md absolute h-24 w-24 group-hover:shadow-blurXl group-hover:shadow-steel/50 group-hover:scale-75 ${
							idx === 0 ? 'group-hover:scale-95 shadow-md' : ''
						} ${
							idx === 1
								? 'scale-[0.975] group-hover:-rotate-6 group-hover:-translate-x-6 group-hover:-translate-y-3'
								: ''
						} ${
							idx === 2
								? 'scale-[0.95] group-hover:rotate-6 group-hover:translate-x-6 group-hover:-translate-y-3'
								: ''
						}`}
						style={{
							zIndex: items.length - idx,
							top: `-${idx * 6}px`,
						}}
					>
						{image.data &&
							image.data.display &&
							image.data.display.data &&
							typeof image.data.display.data === 'object' &&
							'image_url' in image.data.display.data && (
								<NftImage name="Kiosk" src={image.data?.display?.data.image_url} size="lg" />
							)}
					</div>
				))}
			<div
				className="absolute right-1.5 bottom-1.5 h-6 w-6 bg-gray-100 flex items-center justify-center text-white rounded-md"
				style={{ zIndex: items?.length }}
			>
				<Text variant="subtitle" weight="medium">
					{items?.length}
				</Text>
			</div>
		</div>
	);
}
