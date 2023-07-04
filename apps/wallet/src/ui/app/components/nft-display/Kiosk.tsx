// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useGetKioskContents } from '@mysten/core';
import { type SuiObjectResponse } from '@mysten/sui.js';

import { useActiveAddress } from '../../hooks';
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
		<div className="relative bg-hero">
			{items
				?.slice(0, 1)
				.reverse()
				.map((image, idx) => (
					<div
						className={`shadow-md rounded-md`}
						style={{ zIndex: items.length - idx, top: `${idx * 5}px`, left: `${idx * 5}px` }}
					>
						{image.data &&
							image.data.display &&
							image.data.display.data &&
							typeof image.data.display.data === 'object' &&
							'image_url' in image.data.display.data && (
								<NftImage name="" src={image.data?.display?.data.image_url} size={'md'} />
							)}
					</div>
				))}
		</div>
	);
}
